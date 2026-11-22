//! The hosted-workspace calls: what the real shell asks its orchestrator for, and how the answers
//! become this port's model.
//!
//! Shapes are the ones recorded in `docs/port-notes.md` §8–9: one endpoint per resource, a POST of
//! one object with an `operation`, and a reply that is the whole slice the caller renders from.
//! Only the calls a reading client needs are here — bootstrap, authorize, projects, threads and
//! `read`. Writes (sending a turn, saving the workspace) belong to the cloud half (§9), not this.

use serde_json::{Value, json};

use crate::model::{Elevation, History, Message, Part, Role, Thread};

/// What a running orchestrator answers to: its loopback port and the launch id every `/api/*` call
/// must carry. Both come out of the boot (`Live::Ready`).
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub port: u16,
    pub launch: String,
}

/// One project as `local-conversations {operation: "projects"}` names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteProject {
    pub id: String,
    pub name: String,
}

/// One thread as `{operation: "threads"}` names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteThread {
    pub id: String,
    pub title: String,
    pub running: bool,
    pub harness: String,
}

/// A transcript, ready to put on screen.
pub struct Transcript {
    pub title: String,
    pub messages: Vec<Message>,
    pub history: History,
    pub elevations: Vec<Elevation>,
}

/// The sidebar's data: every project, each with its threads.
pub struct Snapshot {
    pub account: String,
    pub projects: Vec<(RemoteProject, Vec<RemoteThread>)>,
}

impl Endpoint {
    /// One `/api/hosted-workspace/<resource>` call. A non-2xx answer is an error carrying the
    /// server's own sentence — `{"error": "Reconnect this local folder to use its tools"}` is a
    /// state the UI draws, not a crash (§8).
    pub fn rpc(&self, resource: &str, body: &Value) -> Result<Value, String> {
        let path = format!("/api/hosted-workspace/{resource}");
        let text = body.to_string();
        let (status, body) = crate::orchestrator::request_body(
            self.port,
            "POST",
            &path,
            Some(&self.launch),
            Some(text.as_str()),
        )
        .map_err(|err| format!("{path}: {err}"))?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|err| format!("{path}: unreadable reply ({err})"))?;
        if !(200..300).contains(&status) {
            let why = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("the desktop workspace is unavailable");
            return Err(why.to_string());
        }
        Ok(value)
    }

    fn local(&self, body: Value) -> Result<Value, String> {
        self.rpc("local-conversations", &body)
    }

    /// `bootstrap` hands back the account and the hosted page's URL; the account is what every
    /// later call needs, and the URL is the thing a native UI would have to replace (§9).
    pub fn bootstrap(&self) -> Result<String, String> {
        let reply = self.rpc("bootstrap", &json!({ "launchId": self.launch }))?;
        string(&reply, "accountId").ok_or_else(|| "bootstrap sent no accountId".to_string())
    }

    pub fn authorize(&self, account: &str) -> Result<(), String> {
        self.local(json!({ "operation": "authorize", "accountId": account }))
            .map(|_| ())
    }

    pub fn projects(&self, account: &str) -> Result<Vec<RemoteProject>, String> {
        let reply =
            self.local(json!({ "operation": "projects", "accountId": account, "offset": 0 }))?;
        Ok(array(&reply, "projects")
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(RemoteProject::parse)
            .collect())
    }

    pub fn threads(&self, account: &str, project: &str) -> Result<Vec<RemoteThread>, String> {
        let reply = self.local(json!({
            "operation": "threads",
            "accountId": account,
            "projectId": project,
            "offset": 0,
        }))?;
        Ok(array(&reply, "threads")
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(RemoteThread::parse)
            .collect())
    }

    /// The sidebar's whole data set: the account, then every project with its threads.
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let account = self.bootstrap()?;
        self.authorize(&account)?;
        let mut projects = Vec::new();
        for project in self.projects(&account)? {
            let threads = self.threads(&account, &project.id).unwrap_or_default();
            projects.push((project, threads));
        }
        Ok(Snapshot { account, projects })
    }

    /// `operation: "read"` — the transcript window. `after: 0` is the start of the thread.
    pub fn read(
        &self,
        account: &str,
        project: &str,
        thread: &str,
        after: u64,
    ) -> Result<Transcript, String> {
        let reply = self.local(json!({
            "operation": "read",
            "accountId": account,
            "projectId": project,
            "threadId": thread,
            "after": after,
        }))?;
        Transcript::parse(&reply).ok_or_else(|| "read sent an unreadable transcript".to_string())
    }
}

impl RemoteProject {
    fn parse(value: &Value) -> Option<RemoteProject> {
        Some(RemoteProject {
            id: string(value, "id")?,
            name: string(value, "name").unwrap_or_else(|| "Untitled project".to_string()),
        })
    }
}

impl RemoteThread {
    fn parse(value: &Value) -> Option<RemoteThread> {
        Some(RemoteThread {
            id: string(value, "id")?,
            title: string(value, "title").unwrap_or_else(|| "New chat".to_string()),
            running: value
                .get("running")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            harness: string(value, "harness").unwrap_or_default(),
        })
    }
}

impl Transcript {
    /// The recorded reply, mapped onto the model: `{thread, history, messages[{seq, role,
    /// parts[]}], elevations[]}`.
    pub fn parse(reply: &Value) -> Option<Transcript> {
        let messages = array(reply, "messages")
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(Message::parse)
            .collect();
        let history = reply.get("history").map(History::parse).unwrap_or_default();
        Some(Transcript {
            title: reply
                .get("thread")
                .and_then(|t| string(t, "title"))
                .unwrap_or_else(|| "New chat".to_string()),
            messages,
            history,
            elevations: array(reply, "elevations")
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(Elevation::parse)
                .collect(),
        })
    }
}

impl Message {
    fn parse(value: &Value) -> Option<Message> {
        let role = match value.get("role").and_then(Value::as_str) {
            Some("user") => Role::User,
            _ => Role::Assistant,
        };
        Some(Message {
            seq: value.get("seq").and_then(Value::as_u64).unwrap_or(0),
            role,
            parts: array(value, "parts")
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(Part::parse)
                .collect(),
        })
    }
}

impl Part {
    /// `parts[].kind` — `text`, `reasoning`, `tool` (with `toolName` and its raw `input`), `ad`.
    /// An unknown kind is dropped rather than drawn as an empty row.
    fn parse(value: &Value) -> Option<Part> {
        match value.get("kind").and_then(Value::as_str)? {
            "text" => Some(Part::Text(string(value, "text").unwrap_or_default())),
            "reasoning" => Some(Part::Reasoning(string(value, "text").unwrap_or_default())),
            "tool" => Some(Part::Tool {
                name: string(value, "toolName").unwrap_or_else(|| "tool".to_string()),
                input: one_line(value.get("input")),
            }),
            "ad" => {
                let ad = value.get("ad")?;
                Some(Part::Ad {
                    title: string(ad, "title").unwrap_or_default(),
                    body: string(ad, "adText").unwrap_or_default(),
                    cta: string(ad, "cta").unwrap_or_else(|| "Learn more".to_string()),
                })
            }
            _ => None,
        }
    }
}

impl Elevation {
    /// The shell's approvals arrive in their own queue. The recorded ones carried no visible text,
    /// so this keeps whatever the payload does name and falls back to the request itself.
    fn parse(value: &Value) -> Option<Elevation> {
        let body = string(value, "body")
            .or_else(|| string(value, "message"))
            .or_else(|| string(value, "reason"))?;
        Some(Elevation {
            title: string(value, "title").unwrap_or_else(|| "Permission request".to_string()),
            body,
        })
    }
}

impl History {
    fn parse(value: &Value) -> History {
        let n = |key: &str| value.get(key).and_then(Value::as_u64);
        let first = n("firstSeq").unwrap_or(0);
        let last = n("lastSeq").unwrap_or(0);
        let total = n("totalMessages").unwrap_or(last);
        History {
            first_seq: first,
            last_seq: last,
            total,
            has_older: value
                .get("hasOlder")
                .and_then(Value::as_bool)
                .unwrap_or(first > 1),
            has_newer: value
                .get("hasNewer")
                .and_then(Value::as_bool)
                .unwrap_or(last < total),
        }
    }
}

/// A tool's `input` as the one line a row can draw: a command, a path, or the JSON flattened
/// enough to read. The real values are small objects (`{command, process_type, timeout_seconds}`).
fn one_line(input: Option<&Value>) -> String {
    let Some(input) = input else {
        return String::new();
    };
    match input {
        Value::String(s) => s.clone(),
        Value::Object(map) => {
            for key in ["command", "path", "query", "url", "file", "pattern"] {
                if let Some(Value::String(s)) = map.get(key) {
                    return s.clone();
                }
            }
            map.iter()
                .filter_map(|(k, v)| match v {
                    Value::String(s) => Some(format!("{k}={s}")),
                    Value::Number(n) => Some(format!("{k}={n}")),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        other => other.to_string(),
    }
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

/// The sidebar's threads for `Thread`: a captured snapshot is written into the model with no
/// project id, so a fixture and a real thread are the same type.
pub fn thread_from(project: &str, remote: &RemoteThread) -> Thread {
    Thread {
        title: remote.title.clone(),
        status: if remote.running {
            crate::model::Status::Working
        } else {
            crate::model::Status::Idle
        },
        messages: Vec::new(),
        elevations: Vec::new(),
        context: 0.0,
        branch: None,
        running: remote.running,
        total_messages: 0,
        remote: Some(crate::model::RemoteRef {
            project: project.to_string(),
            thread: remote.id.clone(),
        }),
    }
}
