//! Seed state: what the shell draws before — and without — the orchestrator.
//!
//! These are *samples*, not anyone's data. The app's own screenshot showed real project and
//! thread names; anything published has to use invented ones, so every name, title and body here
//! is fictional. The shape is what matters: three projects with threads in each of the states the
//! sidebar can draw, three empty ones, and one thread long enough to scroll.

use crate::model::{Act, Item, Project, Status, Thread};

fn t(title: &str, status: Status, context: f32, branch: Option<&str>, items: Vec<Item>) -> Thread {
    Thread {
        title: title.into(),
        status,
        items,
        context,
        branch: branch.map(Into::into),
        running: status == Status::Working || status == Status::NeedsYou,
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
                    Item::User(
                        "can the upload path retry a dropped connection instead of failing the whole batch?"
                            .into(),
                    ),
                    Item::Activity(Act::Read, "Read a file".into()),
                    Item::Agent(
                        "The client retries at the request level, but a batch upload drops the whole group when one part fails. Two options: retry each part with backoff, or checkpoint the batch and resume. The first is smaller."
                            .into(),
                    ),
                    Item::Activity(Act::Run, "Ran a tool, ran a command".into()),
                    Item::Activity(Act::Read, "Read a file".into()),
                    Item::Agent(
                        "The retry helper already carries exponential backoff and jitter, so this is a call site change rather than new machinery. Writing the test first, then the change."
                            .into(),
                    ),
                    Item::Activity(Act::Edit, "Edited a file, ran a command".into()),
                    Item::Agent(
                        "One test still fails: the retry budget is spent before the last part ever attempts. Asking to raise the budget rather than widen it silently."
                            .into(),
                    ),
                    Item::Approval(
                        "Permission request".into(),
                        "Raise the upload retry budget from 3 to 5 attempts and re-run the failing test."
                            .into(),
                    ),
                ],
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
                        Item::User("work through the pending TODOs in the repo".into()),
                        Item::Activity(Act::Search, "Searched 214 files".into()),
                        Item::Agent(
                            "Nine TODOs are real work and four are notes to self. Starting with the scheduler's unimplemented preemption branch."
                                .into(),
                        ),
                    ],
                ),
                t("New chat", Status::Idle, 0.0, None, vec![]),
            ],
        },
        Project {
            name: "orbit-api".into(),
            threads: vec![
                t("Test PR on sync branch", Status::Done, 62.0, Some("test/sync"), vec![]),
                t("New chat", Status::Idle, 0.0, None, vec![]),
            ],
        },
        Project {
            name: "ledger-go".into(),
            threads: vec![t("Port the router to Go", Status::Failed, 12.0, Some("main"), vec![])],
        },
        Project {
            name: "clockwork".into(),
            threads: vec![t("Token budget discovery", Status::Done, 8.0, None, vec![])],
        },
        Project {
            name: "mesh-net".into(),
            threads: vec![t("Distributed inference notes", Status::Idle, 4.0, None, vec![])],
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
