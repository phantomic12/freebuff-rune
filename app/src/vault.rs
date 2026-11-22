//! The auth vault: the one thing the orchestrator needs from Electron that a native shell can
//! provide itself.
//!
//! `docs/port-notes.md` §10 ends on this: desktop sessions are `safeStorage` ciphertext in
//! `{statePath}.auth-secrets.json`, and the orchestrator never decrypts them itself — it asks the
//! Electron main process over a loopback bridge (`FREEBUFF_MCP_CONSENT_PORT` +
//! `FREEBUFF_MCP_CONSENT_TOKEN` → `POST /secret`). A launch without that bridge is signed out by
//! construction. This module is that bridge, natively: the same routes, the same token, the same
//! key namespace, with the ciphertext opened by DPAPI (which is what `safeStorage` is on Windows).
//!
//! Scope is deliberate: only the `auth/` namespace is served (never a connector's OAuth token),
//! and `/confirm` — the approval dialog for MCP connectors — reports itself unavailable rather
//! than pretending to be a window nobody can click.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where a running vault listens, and the bearer token that opens it. Both go into the
/// orchestrator's environment, exactly as they do from Electron.
#[derive(Clone, Debug)]
pub struct Vault {
    pub port: u16,
    pub token: String,
}

/// The file beside `state.json` that both sides agree on (`authSecretsPath` upstream).
pub fn secrets_path(state: &Path) -> PathBuf {
    let mut name = state.file_name().unwrap_or_default().to_os_string();
    name.push(".auth-secrets.json");
    state.with_file_name(name)
}

impl Vault {
    /// Start the bridge on a free loopback port. `None` when this host cannot open the vault's
    /// ciphertext — the orchestrator then behaves as if it were launched without Electron, which is
    /// the honest outcome rather than a vault that answers "no session" to everything.
    pub fn start(state: PathBuf) -> Option<Vault> {
        if !cipher_ready() {
            return None;
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
        let port = listener.local_addr().ok()?.port();
        let token = crate::orchestrator::random_token();
        let vault = Vault {
            port,
            token: token.clone(),
        };
        let secrets = Arc::new(secrets_path(&state));
        eprintln!(
            "freebuff-rune: auth vault listening on 127.0.0.1:{port} (session file {})",
            secrets.display()
        );
        std::thread::spawn(move || serve(listener, token, secrets));
        Some(vault)
    }
}

fn serve(listener: TcpListener, token: String, secrets: Arc<PathBuf>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let token = token.clone();
        let secrets = secrets.clone();
        // One request per connection: the caller is a single local client that retries.
        std::thread::spawn(move || {
            let _ = answer(stream, &token, &secrets);
        });
    }
}

fn answer(stream: TcpStream, token: &str, secrets: &Path) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut authorized = false;
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        // Header names are case-insensitive; the value is not — the token is base64url and can
        // hold capitals, so only the "Bearer" prefix may be folded.
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            match name.trim().to_ascii_lowercase().as_str() {
                "authorization" => {
                    authorized = value
                        .strip_prefix("Bearer ")
                        .or_else(|| value.strip_prefix("bearer "))
                        .is_some_and(|presented| presented == token);
                }
                "content-length" => length = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let mut body = vec![0; length.min(64 * 1024)];
    if length > 0 {
        reader.read_exact(&mut body)?;
    }

    let reply = route(&method, &path, authorized, &body, secrets);
    let text = reply.to_string();
    write!(
        writer,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    )?;
    writer.flush()
}

/// The routes, with the same answers the Electron bridge gives. `status` rides in the body's
/// `code` field because every one of them is a 200 at the HTTP level in this minimal server — the
/// orchestrator reads the JSON, and a body it can parse is worth more than a status line it cannot.
fn route(
    method: &str,
    path: &str,
    authorized: bool,
    body: &[u8],
    secrets: &Path,
) -> serde_json::Value {
    let prefix = serde_json::json!({});
    let _ = prefix;
    if !authorized {
        eprintln!("freebuff-rune: auth vault refused a call without its token");
        return serde_json::json!({"error": {"kind": "bad_request", "message": "missing or invalid token"}});
    }
    if method == "GET" && path == "/healthz" {
        return serde_json::json!({"ok": true});
    }
    if method != "POST" {
        return serde_json::json!({"error": {"kind": "bad_request", "message": format!("unknown route {path}")}});
    }
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(body) else {
        return serde_json::json!({"error": {"kind": "bad_request", "message": "invalid JSON body"}});
    };
    if path == "/confirm" {
        // An approval the shell has no dialog for must fail loudly, not hang the orchestrator.
        return serde_json::json!({"error": {"kind": "unavailable", "message": "this shell has no approval dialog"}});
    }
    if path != "/secret" {
        return serde_json::json!({"error": {"kind": "bad_request", "message": format!("unknown route {path}")}});
    }
    secret(&body, secrets)
}

fn secret(body: &serde_json::Value, secrets: &Path) -> serde_json::Value {
    let op = body.get("op").and_then(|v| v.as_str()).unwrap_or_default();
    let key = body.get("key").and_then(|v| v.as_str()).unwrap_or_default();
    // The bridge's own rule: an `auth/` + sha256-hex key, or nothing. A connector's token is not
    // this shell's business, and a key outside the namespace is refused before the file is read.
    let is_auth = key
        .strip_prefix("auth/")
        .is_some_and(|hash| hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()));
    if !is_auth {
        return serde_json::json!({"error": {"kind": "bad_request", "message": "key is not in a namespace this shell serves"}});
    }
    match op {
        "get" => match read_secret(secrets, key) {
            Some(value) => serde_json::json!({ "value": value }),
            None => {
                eprintln!(
                    "freebuff-rune: the session vault has no entry for {}…",
                    &key[..13]
                );
                serde_json::json!({ "value": null })
            }
        },
        "set" | "delete" => {
            // Reads are the whole job here: writing means being the place a sign-in is stored, which
            // this shell does not do yet. Saying so is better than a silent failure to persist.
            serde_json::json!({"error": {"kind": "unavailable", "message": "this shell reads the desktop session but does not sign in"}})
        }
        _ => {
            serde_json::json!({"error": {"kind": "bad_request", "message": "op must be get, set or delete"}})
        }
    }
}

/// One entry: `{ "auth/<hash>": "<base64 ciphertext>" }` → the plaintext the orchestrator wants.
fn read_secret(file: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let all: serde_json::Value = serde_json::from_str(&text).ok()?;
    let encoded = all.get(key).and_then(|v| v.as_str())?;
    let blob = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).ok()?;
    String::from_utf8(decrypt(&blob)?).ok()
}

/// `safeStorage` on Windows is Chromium's os_crypt: values are `v10` + 12-byte nonce + AES-256-GCM
/// ciphertext and tag, under a key that `Local State` holds DPAPI-encrypted. Same user, same key —
/// which is why a native shell can read what the installed app wrote, and a different user cannot.
fn decrypt(blob: &[u8]) -> Option<Vec<u8>> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    if blob.len() < 15 || !(blob.starts_with(b"v10") || blob.starts_with(b"v11")) {
        return None;
    }
    let key = oscrypt_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;
    cipher.decrypt(blob[3..15].into(), &blob[15..]).ok()
}

/// The AES key, once per process: `os_crypt.encrypted_key` from the app's `Local State`, minus the
/// `DPAPI` tag, opened with DPAPI.
fn oscrypt_key() -> Option<[u8; 32]> {
    static KEY: std::sync::OnceLock<Option<[u8; 32]>> = std::sync::OnceLock::new();
    *KEY.get_or_init(raw_oscrypt_key)
}

fn raw_oscrypt_key() -> Option<[u8; 32]> {
    let path = local_state_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    let state: serde_json::Value = serde_json::from_str(&text).ok()?;
    let encoded = state
        .get("os_crypt")
        .and_then(|c| c.get("encrypted_key"))
        .and_then(|v| v.as_str())?;
    let blob = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).ok()?;
    let wrapped = blob.strip_prefix(b"DPAPI".as_slice())?;
    let plain = unprotect(wrapped)?;
    plain.try_into().ok()
}

/// Electron keeps this in `app.getPath('userData')`, which is the packaged product's name. The
/// install's name is checked first; the scan is the fallback for a build that names itself
/// differently, and it only looks at directories that look like this app.
fn local_state_path() -> Option<PathBuf> {
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from)?;
    for name in ["Freebuff", "@codebufffreebuff-desktop", "freebuff-desktop"] {
        let candidate = roaming.join(name).join("Local State");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = std::fs::read_dir(&roaming).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.contains("freebuff") && !name.contains("codebuff") {
            continue;
        }
        let candidate = entry.path().join("Local State");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Whether this host can open the vault at all: Windows (DPAPI + Chromium's os_crypt key). On other
/// platforms the vault reports itself unavailable rather than half-working.
fn cipher_ready() -> bool {
    cfg!(windows) && oscrypt_key().is_some()
}

#[cfg(windows)]
fn unprotect(blob: &[u8]) -> Option<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        );
        if ok == 0 {
            return None;
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        // The blob's buffer is the system's, not ours.
        windows_sys::Win32::Foundation::LocalFree(output.pbData as *mut core::ffi::c_void);
        Some(bytes)
    }
}

#[cfg(not(windows))]
fn unprotect(_blob: &[u8]) -> Option<Vec<u8>> {
    None
}

/// The key both sides hash to: `auth/` + sha256 hex of the API host. The orchestrator computes it
/// itself; this shell only checks the shape, so this exists for the tests that pin the shape.
#[cfg(test)]
pub fn auth_key(host: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(host.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("auth/{hex}")
}
