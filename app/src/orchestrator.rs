//! The shell's half of Freebuff Desktop's boot contract.
//!
//! The shipped app is two processes. Electron is only the shell; the data lives in a Bun
//! orchestrator (`resources/orchestrator/orchestrator.js`) that boots, prints its progress on
//! stdout and then serves a loopback HTTP API. `extracted/app/electron/main.cjs` and
//! `orchestrator-startup.cjs` define that contract, and this module implements the same one:
//!
//! 1. bind a free loopback port and pass it as `PORT`;
//! 2. give the child a profile (`FREEBUFF_DESKTOP_STATE_PATH`) and a *lifetime* server, so an
//!    orchestrator whose shell died tears itself down instead of outliving it;
//! 3. hand it this launch's secrets on the first stdin line, never in the environment
//!    (`src/server/launch-bootstrap.ts` in the bundle);
//! 4. watch `[orchestrator-phase] <step>` and `[orchestrator-ready] {...}` on stdout, then poll
//!    `/healthz` — 30 s from spawn to the ready line, 60 s from there to health, as upstream;
//! 5. send `x-freebuff-launch-id` on every `/api/` call, because the server refuses its own
//!    API without it once it has a launch id.
//!
//! Everything here is deliberately dependency-light and testable without a child process: the
//! parsers, the HTTP reader and the lifetime handshake are pure functions.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, channel};
use std::thread;
use std::time::{Duration, Instant};

const PHASE_PREFIX: &str = "[orchestrator-phase] ";
const READY_PREFIX: &str = "[orchestrator-ready] ";
const LAUNCH_TOKEN_HEADER: &str = "x-freebuff-launch-id";
/// Spawn → ready line, then ready line → first healthy `/healthz` (upstream's two budgets).
const READY_BUDGET: Duration = Duration::from_secs(30);
const HEALTH_BUDGET: Duration = Duration::from_secs(60);
const HEALTH_RETRY: Duration = Duration::from_millis(300);
/// The orchestrator gives up on a lifetime socket after 10 s of silence; three pings fit.
const PING_EVERY: Duration = Duration::from_secs(3);

/// What the sidebar shows about the orchestrator, and the only thing a background thread writes
/// into the state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Live {
    /// No orchestrator was started (headless snapshot, `--no-orchestrator`, or no install found).
    #[default]
    Off,
    /// The child is up and booting; the string is its last reported phase.
    Booting(String),
    /// `/healthz` answered.
    Ready { port: u16, pid: u32, authed: bool },
    /// The child never booted, or exited.
    Failed(String),
}

impl Live {
    /// The line the sidebar footer draws.
    pub fn label(&self) -> String {
        match self {
            Live::Off => "Orchestrator off".to_string(),
            Live::Booting(phase) => format!("Starting · {phase}"),
            Live::Ready { authed: true, .. } => "Orchestrator ready".to_string(),
            Live::Ready { authed: false, .. } => "Orchestrator · signed out".to_string(),
            Live::Failed(_) => "Orchestrator failed".to_string(),
        }
    }

    /// A failure's own words, short enough for a 254-point sidebar. The orchestrator's last stderr
    /// line is the useful half of a failure, so it gets its own line under the label; the two
    /// cases worth naming outright are the profile lock (another Freebuff is running) and the
    /// exit code, which on its own tells nobody anything.
    pub fn reason(&self) -> Option<String> {
        let Live::Failed(why) = self else { return None };
        let line = why.lines().last().unwrap_or(why).trim();
        let line = line.strip_prefix("[orchestrator] ").unwrap_or(line);
        if line.contains("already using this Desktop state profile") {
            return Some("Another Freebuff has this profile".to_string());
        }
        let line = match line.split_once(": ") {
            Some((head, tail)) if head.contains("exited with code") => tail,
            _ => line,
        };
        Some(match line.char_indices().nth(36) {
            Some((cut, _)) => format!("{}…", &line[..cut]),
            None => line.to_string(),
        })
    }

    /// The same thing for a screen reader, where the short label loses the port and the reason.
    pub fn detail(&self) -> String {
        match self {
            Live::Off => "Freebuff orchestrator is not running".to_string(),
            Live::Booting(phase) => format!("Freebuff orchestrator starting, phase {phase}"),
            Live::Ready { port, pid, authed } => format!(
                "Freebuff orchestrator ready on port {port} as process {pid}, {}",
                if *authed { "signed in" } else { "signed out" }
            ),
            Live::Failed(why) => format!("Freebuff orchestrator failed: {why}"),
        }
    }

    /// The dot's colour. Ready-but-signed-out is a warning: the shell is up, but nothing works
    /// until someone signs in.
    pub fn tone(&self) -> Tone {
        match self {
            Live::Off => Tone::Idle,
            Live::Booting(_) => Tone::Work,
            Live::Ready { authed: true, .. } => Tone::Good,
            Live::Ready { authed: false, .. } => Tone::Warn,
            Live::Failed(_) => Tone::Bad,
        }
    }
}

/// What the footer's dot should say without words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Idle,
    Work,
    Good,
    Warn,
    Bad,
}

/// Where the orchestrator and its runtime live on this machine.
pub struct Config {
    pub bun: PathBuf,
    pub entry: PathBuf,
    pub dir: PathBuf,
    pub state: PathBuf,
    pub wasm: Option<PathBuf>,
}

impl Config {
    /// The installed desktop app's own files. `FREEBUFF_RESOURCES_DIR`, `FREEBUFF_BUN_PATH` and
    /// `FREEBUFF_DESKTOP_STATE_PATH` override each part, matching upstream's overrides so a test
    /// or a second profile can be pointed at a copy.
    pub fn discover() -> Option<Config> {
        let resources = match std::env::var_os("FREEBUFF_RESOURCES_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => installed_resources(env_home()?)?,
        };
        let dir = resources.join("orchestrator");
        let entry = dir.join("orchestrator.js");
        let bun = std::env::var_os("FREEBUFF_BUN_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| resources.join("bun").join(bun_name()));
        if !entry.is_file() || !bun.is_file() {
            return None;
        }
        let wasm = dir.join("wasm");
        Some(Config {
            state: state_path(),
            bun,
            entry,
            dir,
            wasm: wasm.is_dir().then_some(wasm),
        })
    }
}

/// Where the installed app keeps `resources/`. Each platform installs an Electron app differently,
/// so the only safe version is to look in the place that platform's installer uses.
fn installed_resources(home: PathBuf) -> Option<PathBuf> {
    const APP: &str = "@codebufffreebuff-desktop";
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
        vec![local.join("Programs").join(APP).join("resources")]
    } else if cfg!(target_os = "macos") {
        vec![
            home.join("Applications")
                .join(APP)
                .join("Contents")
                .join("Resources"),
            PathBuf::from("/Applications")
                .join(APP)
                .join("Contents")
                .join("Resources"),
        ]
    } else {
        vec![
            home.join(".local")
                .join("share")
                .join(APP)
                .join("resources"),
            PathBuf::from("/opt").join(APP).join("resources"),
            PathBuf::from("/usr")
                .join("lib")
                .join(APP)
                .join("resources"),
        ]
    };
    candidates.into_iter().find(|p| p.is_dir())
}

fn bun_name() -> &'static str {
    if cfg!(windows) { "bun.exe" } else { "bun" }
}

fn env_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// The installed app's profile: `~/.config/freebuff-desktop/state.json`, as
/// `desktop-profile.cjs::realStatePath` computes it on every platform.
pub fn state_path() -> PathBuf {
    if let Some(p) = std::env::var_os("FREEBUFF_DESKTOP_STATE_PATH") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".config")
        .join("freebuff-desktop")
        .join("state.json")
}

/// Boot an orchestrator on a background thread, reporting each state it reaches. Returns at once;
/// the child lives as long as this process holds the lifetime socket.
pub fn launch<F>(cfg: Config, emit: F)
where
    F: Fn(Live) + Send + 'static,
{
    thread::spawn(move || run(cfg, emit));
}

fn run<F: Fn(Live)>(cfg: Config, emit: F) {
    let mut last = Live::Off;
    let mut say = |live: Live| {
        if live != last {
            last = live.clone();
            emit(live);
        }
    };

    let port = match free_port() {
        Ok(port) => port,
        Err(err) => return say(Live::Failed(format!("no loopback port: {err}"))),
    };
    let launch_id = random_token();
    let lifetime_token = random_token();
    let lifetime_token_server = lifetime_token.clone();
    let lifetime = match Lifetime::start(lifetime_token.clone()) {
        Ok(server) => server,
        Err(err) => return say(Live::Failed(format!("no lifetime server: {err}"))),
    };

    // Answers the orchestrator's challenge with an HMAC only this process and the child know, so
    // no other local program can keep it alive after this shell is gone.
    let _keepalive = thread::spawn(move || lifetime.serve(lifetime_token_server));

    say(Live::Booting("spawn".into()));
    let mut cmd = Command::new(&cfg.bun);
    cmd.arg(&cfg.entry)
        .current_dir(&cfg.dir)
        .env("PORT", port.to_string())
        .env("FREEBUFF_SHELL_LIFETIME_PORT", lifetime_port().to_string())
        .env("FREEBUFF_DESKTOP_STATE_PATH", &cfg.state)
        .env("FREEBUFF_PROFILE_LOCK_WAIT_MS", "10000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(wasm) = &cfg.wasm {
        cmd.env("CODEBUFF_WASM_DIR", wasm).env(
            "CODEBUFF_TREE_SITTER_WASM_PATH",
            wasm.join("tree-sitter.wasm"),
        );
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return say(Live::Failed(format!("spawn {}: {err}", cfg.bun.display()))),
    };

    // Secrets ride the first stdin line, not the environment: another process of the same user can
    // read a child's starting environment, so upstream keeps the launch id out of it too.
    if let Some(stdin) = child.stdin.as_mut() {
        let line = format!(
            "{{\"shellLifetimeToken\":\"{lifetime_token}\",\"launchId\":\"{launch_id}\"}}\n"
        );
        let _ = stdin.write_all(line.as_bytes());
        let _ = stdin.flush();
    }

    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (tx, rx) = channel::<Line>();
    spawn_lines(BufReader::new(stdout), tx.clone(), false);
    spawn_lines(BufReader::new(stderr), tx, true);

    let mut stderr_tail = String::new();
    let mut deadline = Instant::now() + READY_BUDGET;
    let mut wait_health = false;
    loop {
        let wait = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(if wait.is_zero() { HEALTH_RETRY } else { wait }) {
            Ok(Line { text, err }) => {
                if err {
                    push_tail(&mut stderr_tail, &text);
                    continue;
                }
                // `listen` means the socket is up; the ready line follows at once. After the
                // ready line the phases are noise, so they stop extending the deadline.
                if let (Some(phase), false) = (parse_phase(&text), wait_health) {
                    say(Live::Booting(phase.to_string()));
                    deadline = Instant::now() + READY_BUDGET;
                }
                if let Some((ready_port, ready_pid)) = parse_ready(&text) {
                    say(Live::Booting("healthz".into()));
                    if !wait_health {
                        deadline = Instant::now() + HEALTH_BUDGET;
                    }
                    wait_health = true;
                    match poll_health(ready_port, &launch_id, deadline) {
                        Ok(authed) => say(Live::Ready {
                            port: ready_port,
                            pid: ready_pid,
                            authed,
                        }),
                        Err(why) => {
                            let _ = child.kill();
                            return say(Live::Failed(why));
                        }
                    }
                    deadline = Instant::now() + Duration::from_secs(3600);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if !wait_health && Instant::now() >= deadline {
                    let _ = child.kill();
                    return say(Live::Failed(
                        "the orchestrator printed no ready line within 30 s".to_string(),
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let code = child.wait().ok().map(|s| s.code().unwrap_or(-1));
    let why = match code {
        Some(0) => "the orchestrator exited".to_string(),
        Some(c) => format!("the orchestrator exited with code {c}"),
        None => "the orchestrator stopped".to_string(),
    };
    say(Live::Failed(match stderr_tail.trim() {
        "" => why,
        tail => format!("{why}: {}", tail.lines().last().unwrap_or(tail).trim()),
    }));
}

struct Line {
    text: String,
    err: bool,
}

fn spawn_lines<R: Read + Send + 'static>(reader: BufReader<R>, tx: mpsc::Sender<Line>, err: bool) {
    thread::spawn(move || {
        for line in reader.lines() {
            let Ok(text) = line else { break };
            if tx.send(Line { text, err }).is_err() {
                break;
            }
        }
    });
}

pub(crate) fn push_tail(tail: &mut String, line: &str) {
    tail.push_str(line);
    tail.push('\n');
    // Upstream keeps ~3000 chars for its failure dialog; the last line is what the label shows.
    if tail.len() > 3000 {
        // Cutting at a byte offset can land inside a multi-byte character — a path with an
        // accent or a title with an emoji is enough — and slicing there panics. Step to a
        // boundary first.
        let mut cut = tail.len() - 3000;
        while !tail.is_char_boundary(cut) {
            cut += 1;
        }
        let keep = tail[cut..].find('\n').map_or(cut, |i| cut + i + 1);
        tail.drain(..keep);
    }
}

/// `[orchestrator-phase] listen` → `listen`.
pub fn parse_phase(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix(PHASE_PREFIX)?;
    let phase = rest.trim();
    // Only the whitelist upstream ships is worth showing; anything else is noise on stdout.
    const PHASES: [&str; 9] = [
        "started",
        "bootstrap",
        "profile_lock",
        "auth",
        "recover_setups",
        "terminals",
        "listen",
        "projects_boot_start",
        "projects_boot_end",
    ];
    PHASES.contains(&phase).then_some(phase)
}

/// `[orchestrator-ready] {"launchId":"…","pid":1,"port":7781}` → `(port, pid)`.
pub fn parse_ready(line: &str) -> Option<(u16, u32)> {
    let rest = line.trim().strip_prefix(READY_PREFIX)?;
    let port = number_after(rest, "\"port\"")?;
    // A port that cannot be one is a misread line, not a boot: `as u16` would silently wrap 70000
    // into 4464 and then poll a stranger's socket.
    let port = u16::try_from(port).ok().filter(|p| *p != 0)?;
    // The pid rides the same line; an older orchestrator that omits it still boots.
    let pid = number_after(rest, "\"pid\"").unwrap_or(0);
    Some((port, pid))
}

/// The last integer after `key` in a small JSON object. The ready line prints
/// `{launchId, pid, port}`, so the top-level `port` is the *last* occurrence; a decoy key nested
/// in an earlier value must not win.
fn number_after(json: &str, key: &str) -> Option<u32> {
    let at = json.rfind(key)?;
    let digits: String = json[at + key.len()..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// `{"authed":false,…}` → `false`. The field is a plain boolean; no need for a JSON crate to read it.
pub fn parse_authed(body: &str) -> bool {
    let Some(at) = body.find("\"authed\"") else {
        return false;
    };
    let after = body[at + 8..].trim_start();
    let value = after.strip_prefix(':').unwrap_or(after).trim_start();
    value.starts_with("true")
}

/// Poll `/healthz` until it answers 200 with `ok`, then report whether the profile is signed in.
fn poll_health(port: u16, launch_id: &str, until: Instant) -> Result<bool, String> {
    loop {
        if let Ok((200, body)) = request(port, "GET", "/healthz", Some(launch_id))
            && (body.trim() == "ok" || body.contains("\"ok\":true"))
        {
            let authed = match request(port, "GET", "/api/auth/status", Some(launch_id)) {
                Ok((200, body)) => parse_authed(&body),
                _ => false,
            };
            return Ok(authed);
        }
        if Instant::now() >= until {
            return Err("the orchestrator never answered /healthz".to_string());
        }
        thread::sleep(HEALTH_RETRY);
    }
}

/// One loopback HTTP/1.1 exchange. Everything here is localhost and tiny: a request line, the
/// launch header, and a body read to EOF (Bun answers with `Connection: close`).
fn request(
    port: u16,
    method: &str,
    path: &str,
    launch_id: Option<&str>,
) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept: application/json, text/plain\r\n"
    );
    if let Some(id) = launch_id {
        req.push_str(&format!("{LAUNCH_TOKEN_HEADER}: {id}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    parse_response(&raw).ok_or_else(|| std::io::Error::other("malformed HTTP response"))
}

/// Split a raw response into its status code and decoded body.
pub fn parse_response(raw: &[u8]) -> Option<(u16, String)> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let body = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let chunked = head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    let text = if chunked {
        dechunk(body)
    } else {
        body.to_vec()
    };
    Some((status, String::from_utf8_lossy(&text).into_owned()))
}

fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let Some(end) = body.windows(2).position(|w| w == b"\r\n") else {
            return out;
        };
        let size = std::str::from_utf8(&body[..end])
            .ok()
            .and_then(|hex| usize::from_str_radix(hex.trim(), 16).ok());
        let Some(size) = size else { return out };
        if size == 0 {
            return out;
        }
        let start = end + 2;
        let stop = (start + size).min(body.len());
        out.extend_from_slice(&body[start..stop]);
        body = body.get(stop + 2..).unwrap_or(&[]);
        if body.is_empty() {
            return out;
        }
    }
}

/// Ask the OS for a free loopback port. The orchestrator binds it a moment later; upstream picks
/// its port the same way and retries on the loser of that race.
fn free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    if getrandom::fill(&mut bytes).is_err() {
        // A timestamp and pid still keep two launches apart; the handshake never crosses a trust
        // boundary that needs more than that.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        bytes[..8].copy_from_slice(&nanos.to_le_bytes());
        bytes[8..12].copy_from_slice(&std::process::id().to_le_bytes());
    }
    base64_url(&bytes)
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn base64_url_decode(text: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text.trim())
        .ok()
}

/// The socket the orchestrator saw, so its `FREEBUFF_SHELL_LIFETIME_PORT` matches. The listener is
/// created before the child starts and kept for the process's life: when this process dies, even
/// from a kill, the socket closes and the orchestrator stops on its own.
struct Lifetime {
    listener: TcpListener,
}

thread_local! {
    static LIFETIME_PORT: std::cell::Cell<u16> = const { std::cell::Cell::new(0) };
}

fn lifetime_port() -> u16 {
    LIFETIME_PORT.with(|p| p.get())
}

impl Lifetime {
    fn start(_token: String) -> std::io::Result<Lifetime> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        LIFETIME_PORT.with(|p| {
            if let Ok(addr) = listener.local_addr() {
                p.set(addr.port());
            }
        });
        Ok(Lifetime { listener })
    }

    /// Upstream's handshake (`shell-lifetime.cjs`): we send a challenge, the orchestrator answers
    /// `clientChallenge:HMAC(token, "client:<challenge>:<clientChallenge>")`, and we answer back
    /// `HMAC(token, "server:<challenge>:<clientChallenge>")`. Only then do pings start. A socket
    /// that closes — our process exiting — is what the orchestrator reads as "the shell is gone".
    fn serve(self, token: String) {
        for stream in self.listener.incoming() {
            let Ok(stream) = stream else { break };
            let token = token.clone();
            thread::spawn(move || keep_alive(stream, &token));
        }
    }
}

fn keep_alive(mut stream: TcpStream, token: &str) {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;

    let challenge = base64_url(&random_bytes(32));
    if stream
        .write_all(format!("{challenge}\n").as_bytes())
        .is_err()
    {
        return;
    }
    let reader = match stream.try_clone() {
        Ok(s) => BufReader::new(s),
        Err(_) => return,
    };
    let mut line = String::new();
    if reader.take(1024).read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    let mut parts = line.trim().split(':');
    let (Some(client), Some(proof), None) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };
    let mac = |message: String| -> Option<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(token.as_bytes()).ok()?;
        mac.update(message.as_bytes());
        Some(mac.finalize().into_bytes().to_vec())
    };
    let Some(expected) = mac(format!("client:{challenge}:{client}")) else {
        return;
    };
    if base64_url_decode(proof).as_deref() != Some(expected.as_slice()) {
        return;
    }
    let Some(reply) = mac(format!("server:{challenge}:{client}")) else {
        return;
    };
    if stream
        .write_all(format!("{}\n", base64_url(&reply)).as_bytes())
        .is_err()
    {
        return;
    }
    loop {
        thread::sleep(PING_EVERY);
        if stream.write_all(b"ping\n").is_err() {
            return;
        }
    }
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    if getrandom::fill(&mut v).is_err() {
        for (i, b) in v.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
    }
    v
}
