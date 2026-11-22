//! The shell's state. The real app holds this in a React store fed by the local orchestrator
//! over HTTP/SSE; here it is plain Rust, so a handler mutates it directly and the next frame
//! diffs against it.

use rune::TextEdit;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Code,
    Chat,
}

/// The state a thread's tab icon and its row in the sidebar both read.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    NeedsYou,
    Done,
    Failed,
}

/// Which mark a tool row draws, derived from the tool's name.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Read,
    Run,
    Edit,
    Search,
    Think,
}

impl Act {
    /// `shell`/`run` draw a terminal, `read_file` a list, and so on. The recorded `toolName`s are
    /// the agent's own (`shell`, `read_file`, …), so the row reads like the tool instead of a
    /// generic one.
    pub fn of(tool: &str) -> Act {
        let t = tool.to_ascii_lowercase();
        if t.contains("shell")
            || t.contains("run")
            || t.contains("command")
            || t.contains("terminal")
        {
            Act::Run
        } else if t.contains("edit")
            || t.contains("write")
            || t.contains("patch")
            || t.contains("apply")
        {
            Act::Edit
        } else if t.contains("search") || t.contains("grep") || t.contains("find") {
            Act::Search
        } else if t.contains("think") || t.contains("reason") || t.contains("plan") {
            Act::Think
        } else {
            Act::Read
        }
    }
}

/// Who a message is from: `messages[].role` in the recorded payload.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    User,
    Assistant,
}

/// One piece of a message. The transcript is parts, not text — the recorded `parts[].kind` is
/// exactly this set, and a renderer that assumes one string per message cannot draw the real UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// Prose.
    Text(String),
    /// Visible thinking. The app folds it to a row until it is opened (`open` in the payload).
    Reasoning(String),
    /// A tool call: the tool's name and the input it was given, one line.
    Tool { name: String, input: String },
    /// A sponsored card carried *inside* the flow, as the recorded `kind: "ad"` part is. The real
    /// one adds `favicon`, `clickUrl`, `impUrl` and `placementId`; this port draws the visible four.
    Ad {
        title: String,
        body: String,
        cta: String,
    },
}

pub struct Message {
    /// Position in the thread, the value the `before`/`after` cursors page by.
    pub seq: u64,
    pub role: Role,
    pub parts: Vec<Part>,
}

/// A request that stops the thread until it is answered. The real app keeps these in the
/// `elevations` queue beside the messages, not inside them.
pub struct Elevation {
    pub title: String,
    pub body: String,
}

/// The pagination block `local-conversations` answers `read` with, in its own terms.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct History {
    pub first_seq: u64,
    pub last_seq: u64,
    pub total: u64,
    pub has_older: bool,
    pub has_newer: bool,
}

/// Where a thread lives for the orchestrator: the ids a `read` call needs. `None` on the fixtures,
/// which exist so the shell can be looked at without a signed-in profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRef {
    pub project: String,
    pub thread: String,
}

/// One thread as the orchestrator named it, kept beside the fixture-shaped fields.
pub struct Thread {
    pub title: String,
    pub status: Status,
    pub messages: Vec<Message>,
    pub elevations: Vec<Elevation>,
    /// The share of the context window in use, for the Overview panel.
    pub context: f32,
    pub branch: Option<String>,
    pub running: bool,
    /// What the thread holds beyond what is loaded — the `totalMessages` of `history`, which can
    /// exceed `messages.len()` exactly as it can in the real payload.
    pub total_messages: u64,
    /// Set when the thread came from the orchestrator; `open_thread` then asks for its transcript.
    pub remote: Option<RemoteRef>,
}

/// A transcript the sync thread was asked for: which thread, by the orchestrator's ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Want {
    pub project: String,
    pub thread: String,
}

/// The live connection to a synced orchestrator: the channel the UI uses to ask for a transcript.
/// Opened once, when the boot says it is signed in; the sync thread holds the account.
pub struct Sync {
    pub wants: std::sync::mpsc::Sender<Want>,
}

impl Thread {
    /// A thread that holds nothing: what an account with no projects has to draw instead of
    /// indexing into an empty vector. It has no remote ids, so opening it asks for nothing.
    pub fn nothing(title: &str) -> Thread {
        Thread {
            title: title.to_string(),
            status: Status::Idle,
            messages: Vec::new(),
            elevations: Vec::new(),
            context: 0.0,
            branch: None,
            running: false,
            total_messages: 0,
            remote: None,
        }
    }

    /// What a `read` would report about this window.
    pub fn history(&self) -> History {
        let first_seq = self.messages.first().map_or(0, |m| m.seq);
        let last_seq = self.messages.last().map_or(0, |m| m.seq);
        let total = self.total_messages.max(last_seq);
        History {
            first_seq,
            last_seq,
            total,
            has_older: first_seq > 1,
            has_newer: last_seq < total,
        }
    }

    /// Page by cursor, as `operation: "read"` does: the messages after `after`, at most `limit` of
    /// them. `after: 0` is the start of the thread.
    pub fn read(&self, after: u64, limit: usize) -> &[Message] {
        let start = self
            .messages
            .iter()
            .position(|m| m.seq > after)
            .unwrap_or(self.messages.len());
        let end = (start + limit).min(self.messages.len());
        &self.messages[start..end]
    }
}

pub struct Project {
    pub name: String,
    pub threads: Vec<Thread>,
}

pub struct State {
    pub dark: bool,
    pub mode: Mode,
    pub settings_open: bool,
    pub panel_open: bool,
    pub sidebar_open: bool,
    pub projects: Vec<Project>,
    /// Which thread owns the transcript: an index into `projects`, then into its threads.
    pub active: (usize, usize),
    /// The threads with a tab in the title bar, in the order they were opened — the app's
    /// `.tabs` list, which holds what you opened rather than every thread in the sidebar.
    pub open_tabs: Vec<(usize, usize)>,
    /// Tabs closed with the keyboard, for `mod+shift+t`. The real app keeps its closed-tab
    /// history in the window, so this is the same habit, bounded so it cannot grow forever.
    pub reopen_stack: Vec<(usize, usize)>,
    pub draft: TextEdit,
    pub model: &'static str,
    pub location: &'static str,
    pub streak_days: u32,
    pub invites: (u32, u32),
    /// What the local orchestrator is doing — the one part of this state a background thread
    /// writes, through `AppHandle::update`.
    pub backend: crate::orchestrator::Live,
    /// Present once the orchestrator is signed in and synced; `None` means the fixtures are what is
    /// on screen.
    pub sync: Option<Sync>,
}

impl State {
    pub fn seed() -> State {
        State {
            dark: true,
            mode: Mode::Code,
            settings_open: false,
            panel_open: true,
            sidebar_open: true,
            projects: crate::fixtures::projects(),
            active: (0, 0),
            open_tabs: vec![(0, 0), (1, 0), (2, 0)],
            reopen_stack: Vec::new(),
            draft: TextEdit::new(""),
            model: "DeepSeek V4.1 Flash Fast High",
            location: "Local",
            streak_days: 8,
            invites: (0, 1),
            backend: crate::orchestrator::Live::Off,
            sync: None,
        }
    }

    pub fn thread(&self) -> &Thread {
        &self.projects[self.active.0].threads[self.active.1]
    }

    pub fn thread_mut(&mut self) -> &mut Thread {
        &mut self.projects[self.active.0].threads[self.active.1]
    }

    /// Send the composer's text as the person's next message. The real app posts it to the
    /// orchestrator; here the agent answers from a fixed line so the UI can be exercised.
    pub fn send(&mut self) {
        let text = self.draft.text.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.draft.clear();
        let (p, t) = self.active;
        let thread = &mut self.projects[p].threads[t];
        let mut seq = thread.messages.last().map_or(0, |m| m.seq);
        // The person's turn, then the agent's: the same two messages the orchestrator would hand
        // back on the next `read`.
        seq += 1;
        thread.messages.push(Message {
            seq,
            role: Role::User,
            parts: vec![Part::Text(text)],
        });
        seq += 1;
        thread.messages.push(Message {
            seq,
            role: Role::Assistant,
            parts: vec![
                Part::Reasoning("Reading the shell's state before answering.".into()),
                Part::Text(
                    "Working on it — the native shell now owns this turn instead of the Chromium renderer."
                        .into(),
                ),
            ],
        });
        thread.total_messages = seq;
        thread.status = Status::Working;
        thread.running = true;
    }

    /// Replace the fixtures with what the orchestrator actually holds. The sidebar's shape is the
    /// same either way, which is the point of the model being the model.
    pub fn apply_snapshot(
        &mut self,
        projects: Vec<(
            crate::remote::RemoteProject,
            Vec<crate::remote::RemoteThread>,
        )>,
        wants: std::sync::mpsc::Sender<Want>,
    ) {
        // Only projects with something to open reach the sidebar; an empty account gets one
        // placeholder, because every renderer indexes `projects[active]` rather than checking.
        self.projects = projects
            .iter()
            .filter(|(_, threads)| !threads.is_empty())
            .map(|(project, threads)| Project {
                name: project.name.clone(),
                threads: threads
                    .iter()
                    .map(|thread| crate::remote::thread_from(&project.id, thread))
                    .collect(),
            })
            .collect();
        if self.projects.is_empty() {
            self.projects.push(Project {
                name: "No projects yet".to_string(),
                threads: vec![Thread::nothing("Nothing open yet")],
            });
        }
        self.active = (0, 0);
        self.open_tabs = (0..self.projects.len().min(3)).map(|p| (p, 0)).collect();
        if self.open_tabs.is_empty() {
            self.open_tabs.push((0, 0));
        }
        self.sync = Some(Sync { wants });
    }

    /// Put a `read` reply on screen. The ids say which thread, because the answer arrives after the
    /// person may have moved on.
    pub fn apply_transcript(
        &mut self,
        project: &str,
        thread: &str,
        transcript: crate::remote::Transcript,
    ) {
        let Some(p) = self.projects.iter().position(|project_| {
            project_.threads.iter().any(|t| {
                t.remote
                    .as_ref()
                    .is_some_and(|r| r.thread == thread && r.project == project)
            })
        }) else {
            return;
        };
        let Some(t) = self.projects[p].threads.iter().position(|t| {
            t.remote
                .as_ref()
                .is_some_and(|r| r.thread == thread && r.project == project)
        }) else {
            return;
        };
        let target = &mut self.projects[p].threads[t];
        target.title = transcript.title;
        target.messages = transcript.messages;
        target.elevations = transcript.elevations;
        target.total_messages = transcript.history.total;
        target.running = transcript.history.has_newer;
        if transcript.history.has_newer {
            target.status = Status::Working;
        } else if !target.elevations.is_empty() {
            target.status = Status::NeedsYou;
        } else if target.status == Status::Working {
            target.status = Status::Done;
        }
    }

    pub fn open_thread(&mut self, p: usize, t: usize) {
        self.active = (p, t);
        self.settings_open = false;
        // A synced thread has a transcript behind it; ask the sync thread for it, once per open.
        if let (Some(sync), Some(remote)) = (&self.sync, self.projects[p].threads[t].remote.clone())
        {
            let _ = sync.wants.send(Want {
                project: remote.project,
                thread: remote.thread,
            });
        }
        if !self.open_tabs.contains(&(p, t)) {
            self.open_tabs.push((p, t));
            // The title bar holds a row of tabs, not the whole project tree.
            if self.open_tabs.len() > 6 {
                self.open_tabs.remove(0);
            }
        }
    }

    pub fn close_tab(&mut self, p: usize, t: usize) {
        if self.open_tabs.len() <= 1 {
            return;
        }
        self.open_tabs.retain(|tab| *tab != (p, t));
        if self.active == (p, t) {
            self.active = self.open_tabs[self.open_tabs.len() - 1];
        }
        if self.reopen_stack.len() >= 10 {
            self.reopen_stack.remove(0);
        }
        self.reopen_stack.push((p, t));
    }

    /// `mod+shift+t`: the most recently closed tab comes back, closing nothing else.
    pub fn reopen_tab(&mut self) -> bool {
        let Some(tab) = self.reopen_stack.pop() else {
            return false;
        };
        if !self.open_tabs.contains(&tab) {
            self.open_tabs.push(tab);
        }
        self.active = tab;
        true
    }

    /// `mod+w` on the focused tab. Another tab takes its place, as in the menu command.
    pub fn close_active_tab(&mut self) {
        self.close_tab(self.active.0, self.active.1);
    }

    /// `mod+t`: a thread of the active project that does not already have a tab. A real new
    /// thread needs the orchestrator (#1685); with the fixtures this opens the project's next one.
    pub fn new_tab(&mut self) {
        // The active project first, then the others in sidebar order: a thread that is not already
        // a tab gets one. Nothing to open when every thread is.
        let order = std::iter::once(self.active.0).chain(0..self.projects.len());
        for p in order {
            for t in 0..self.projects[p].threads.len() {
                if !self.open_tabs.contains(&(p, t)) {
                    self.open_tabs.push((p, t));
                    self.active = (p, t);
                    return;
                }
            }
        }
    }

    pub fn tab_thread(&self, p: usize, t: usize) -> &Thread {
        &self.projects[p].threads[t]
    }
}
