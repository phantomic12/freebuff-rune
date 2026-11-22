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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct History {
    pub first_seq: u64,
    pub last_seq: u64,
    pub total: u64,
    pub has_older: bool,
    pub has_newer: bool,
}

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
}

impl Thread {
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

    pub fn open_thread(&mut self, p: usize, t: usize) {
        self.active = (p, t);
        self.settings_open = false;
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
