//! Seed state: what the shell draws before — and without — the orchestrator.
//!
//! These are *samples*, not anyone's data. The app's own screenshot showed real project and
//! thread names; anything published has to use invented ones, so every name, title and body here
//! is fictional. The shape is what matters, and the shape is the recorded one: three projects with
//! threads in each of the states the sidebar can draw, three empty ones, and one thread long
//! enough to scroll — its messages carrying the four part kinds a real transcript carries.

use crate::model::{Elevation, Message, Part, Project, Role, Status, Thread};

fn msg(seq: u64, role: Role, parts: Vec<Part>) -> Message {
    Message { seq, role, parts }
}

fn t(
    title: &str,
    status: Status,
    context: f32,
    branch: Option<&str>,
    messages: Vec<Message>,
    elevations: Vec<Elevation>,
) -> Thread {
    let total_messages = messages.len() as u64;
    Thread {
        title: title.into(),
        status,
        messages,
        elevations,
        context,
        branch: branch.map(Into::into),
        running: status == Status::Working || status == Status::NeedsYou,
        total_messages,
        remote: None,
    }
}

fn text(body: &str) -> Part {
    Part::Text(body.into())
}

fn tool(name: &str, input: &str) -> Part {
    Part::Tool {
        name: name.into(),
        input: input.into(),
    }
}

pub fn projects() -> Vec<Project> {
    vec![
        Project {
            name: "atlas-web".into(),
            threads: vec![t(
                "Add retry to the upload endpoint",
                Status::NeedsYou,
                20.0,
                Some("main"),
                vec![
                    msg(
                        1,
                        Role::User,
                        vec![text(
                            "can the upload path retry a dropped connection instead of failing the whole batch?",
                        )],
                    ),
                    msg(
                        2,
                        Role::Assistant,
                        vec![
                            Part::Reasoning(
                                "The client retries at the request level. A batch upload drops the whole group when one part fails, so the retry has to move inside the batch.".into(),
                            ),
                            tool("read_file", "src/upload/batch.ts"),
                            text(
                                "The client retries at the request level, but a batch upload drops the whole group when one part fails. Two options: retry each part with backoff, or checkpoint the batch and resume. The first is smaller.",
                            ),
                        ],
                    ),
                    msg(
                        3,
                        Role::Assistant,
                        vec![
                            tool("shell", "npm test -- upload"),
                            tool("read_file", "src/upload/retry.ts"),
                            text(
                                "The retry helper already carries exponential backoff and jitter, so this is a call site change rather than new machinery. Writing the test first, then the change.",
                            ),
                            Part::Ad {
                                title: "Your ad could be here".into(),
                                body: "Freebuff is ad-supported. In the app this card arrives as a part of the conversation, from the orchestrator.".into(),
                                cta: "Learn more".into(),
                            },
                        ],
                    ),
                    msg(
                        4,
                        Role::Assistant,
                        vec![
                            tool("edit_file", "src/upload/batch.ts"),
                            text(
                                "One test still fails: the retry budget is spent before the last part ever attempts. Asking to raise the budget rather than widen it silently.",
                            ),
                        ],
                    ),
                ],
                vec![Elevation {
                    title: "Permission request".into(),
                    body: "Raise the upload retry budget from 3 to 5 attempts and re-run the failing test."
                        .into(),
                }],
            )],
        },
        Project {
            name: "beam-engine".into(),
            threads: vec![
                t(
                    "Clear the pending TODOs",
                    Status::Working,
                    41.0,
                    Some("dev"),
                    vec![
                        msg(1, Role::User, vec![text("work through the pending TODOs in the repo")]),
                        msg(
                            2,
                            Role::Assistant,
                            vec![
                                tool("search", "TODO --glob '*.rs'"),
                                text(
                                    "Nine TODOs are real work and four are notes to self. Starting with the scheduler's unimplemented preemption branch.",
                                ),
                            ],
                        ),
                    ],
                    Vec::new(),
                ),
                t("New chat", Status::Idle, 0.0, None, vec![], Vec::new()),
            ],
        },
        Project {
            name: "orbit-api".into(),
            threads: vec![
                t(
                    "Test PR on sync branch",
                    Status::Done,
                    62.0,
                    Some("test/sync"),
                    vec![],
                    Vec::new(),
                ),
                t("New chat", Status::Idle, 0.0, None, vec![], Vec::new()),
            ],
        },
        Project {
            name: "ledger-go".into(),
            threads: vec![t(
                "Port the router to Go",
                Status::Failed,
                12.0,
                Some("main"),
                vec![],
                Vec::new(),
            )],
        },
        Project {
            name: "clockwork".into(),
            threads: vec![t(
                "Token budget discovery",
                Status::Done,
                8.0,
                None,
                vec![],
                Vec::new(),
            )],
        },
        Project {
            name: "mesh-net".into(),
            threads: vec![t(
                "Distributed inference notes",
                Status::Idle,
                4.0,
                None,
                vec![],
                Vec::new(),
            )],
        },
        Project {
            name: "pixel-lab".into(),
            threads: vec![],
        },
        Project {
            name: "loom-work".into(),
            threads: vec![],
        },
        Project {
            name: "echo-bot".into(),
            threads: vec![],
        },
    ]
}

/// The sponsored card above the composer. Freebuff is ad-supported and the slot is part of the
/// product; the port has no creative feed yet, so this is placeholder copy in the app's own voice.
pub const AD_TITLE: &str = "Your ad could be here";
pub const AD_BODY: &str = "Freebuff is ad-supported. In the app this slot carries a sponsored card from the orchestrator.";
pub const AD_CTA: &str = "Learn more";
