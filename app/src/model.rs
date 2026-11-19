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

/// Which tool an activity row names, so it can draw the matching mark.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Read,
    Run,
    Edit,
    Search,
    Think,
}

/// One turn in a transcript.
pub enum Item {
    /// The person's message: a bubble, right-aligned.
    User(String),
    /// The agent's prose: plain text at the transcript's own measure.
    Agent(String),
    /// A collapsed tool row: `Read a file`, `Ran a command`.
    Activity(Act, String),
    /// A request that stops the thread until it is answered.
    Approval(String, String),
}

pub struct Thread {
    pub title: String,
    pub status: Status,
    pub items: Vec<Item>,
    /// The share of the context window in use, for the Overview panel.
    pub context: f32,
    pub branch: Option<String>,
    pub running: bool,
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
        thread.items.push(Item::User(text));
        thread
            .items
            .push(Item::Activity(Act::Think, "Thinking".into()));
        thread.items.push(Item::Agent(
            "Working on it — the native shell now owns this turn instead of the Chromium renderer."
                .into(),
        ));
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
