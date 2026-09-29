//! The evaluation set's tasks (R-19): Scenario A steps A1–A4 and A8, and
//! Scenario H8 (reusing building blocks when they fit, and only then), each
//! with a starting model, the Operator's messages, scripted answers and the
//! checks that grade the resulting System State. Task definitions are code
//! and are committed; results never are (§8.3).

use crate::{Check, Run};
use agq_language::ElementKind as Kind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LockPolicy {
    /// The Operator allows a change to locked elements.
    Allow,
    /// The Operator refuses it.
    Refuse,
}

pub struct Task {
    pub id: &'static str,
    /// The model the task starts from (SysML text).
    pub start: &'static str,
    /// Qualified names locked at the start.
    pub locked: &'static [&'static str],
    /// The Operator's messages, one turn each.
    pub messages: &'static [&'static str],
    /// Scripted answers: the first whose keyword the question contains.
    pub answers: &'static [(&'static str, &'static str)],
    pub lock_policy: LockPolicy,
    pub checks: Vec<Check>,
    /// A question asked in words gets the scripted answer as the next
    /// message; off for tasks that ask only for an answer.
    pub follow_up: bool,
}

impl Task {
    /// The scripted Operator's answer to a question: an option or text that
    /// matches a keyword, or "decide as you think best".
    pub fn answer(&self, question: &str, options: &[String]) -> String {
        let question = question.to_lowercase();
        for (keyword, answer) in self.answers {
            if question.contains(keyword)
                || options.iter().any(|o| o.to_lowercase().contains(keyword))
            {
                return options
                    .iter()
                    .find(|option| option.to_lowercase().contains(&answer.to_lowercase()))
                    .cloned()
                    .unwrap_or_else(|| answer.to_string());
            }
        }
        "Decide as you think best, and tell me what you chose.".to_string()
    }
}

const EMPTY: &str = "package UrlShortener;";

/// The Scenario A model (models/url-shortener).
pub const FULL: &str = include_str!("../../../../models/url-shortener/UrlShortener.sysml");

/// The URL shortener without click statistics.
const CORE: &str = r#"package UrlShortener {
    doc /* A small web service that turns long URLs into short codes and
         * redirects visitors to the long URL. */
    private import ScalarValues::*;
    item def ShortenRequest { attribute longUrl : String; }
    item def ShortLink { attribute code : String; attribute longUrl : String; }
    item def ResolveRequest { attribute code : String; }
    item def Redirect { attribute location : String; }
    port def ShortenPort { in item request : ShortenRequest; out item link : ShortLink; }
    port def ResolvePort { in item request : ResolveRequest; out item redirect : Redirect; }
    port def LinkStorePort { in item save : ShortLink; in item lookup : ResolveRequest; out item found : ShortLink; }
    interface def LinkStorage { end port client : ~LinkStorePort; end port store : LinkStorePort; }
    part def HttpApi {
        doc /* Accepts shorten and resolve requests over HTTP. */
        port shorten : ShortenPort;
        port resolve : ResolvePort;
        port storage : ~LinkStorePort;
    }
    part def LinkStore {
        doc /* Persists short links and looks them up by code. */
        port links : LinkStorePort;
    }
    part def UrlShortenerService {
        part api : HttpApi;
        part store : LinkStore;
        interface storage : LinkStorage connect api.storage to store.links;
    }
    part shortener : UrlShortenerService;
}"#;

/// The core without the interface between the API and the store.
const UNWIRED: &str = r#"package UrlShortener {
    private import ScalarValues::*;
    item def ShortLink { attribute code : String; attribute longUrl : String; }
    item def ResolveRequest { attribute code : String; }
    port def LinkStorePort { in item save : ShortLink; in item lookup : ResolveRequest; out item found : ShortLink; }
    part def HttpApi { port storage : ~LinkStorePort; }
    part def LinkStore { port links : LinkStorePort; }
    part def UrlShortenerService {
        part api : HttpApi;
        part store : LinkStore;
    }
}"#;

/// A model with two mistakes: a type that does not exist and ports that do
/// not fit.
const BROKEN: &str = r#"package UrlShortener {
    private import ScalarValues::*;
    item def ShortLink { attribute code : String; }
    port def LinkStorePort { in item save : ShortLink; }
    part def HttpApi { port storage : LinkStorePort; }
    part def LinkStore { port links : LinkStorePort; attribute capacity : Count; }
    part def UrlShortenerService {
        part api : HttpApi;
        part store : LinkStore;
        connection storage connect api.storage to store.links;
    }
}"#;

fn changed(run: &Run) -> bool {
    run.applied > 0
}
fn unchanged(run: &Run) -> bool {
    run.applied == 0 && run.text() == run.start_text
}
fn no_problems(run: &Run) -> bool {
    run.problems() == 0
}
fn no_new_problems(run: &Run) -> bool {
    run.problems() <= run.start_problems
}
fn asked(run: &Run) -> bool {
    run.asked()
}

/// A usage typed by the project's copy of the Library block `name`.
fn uses_block(run: &Run, name: &str) -> bool {
    let tree = run.state().tree();
    tree.walk().into_iter().any(|id| {
        tree[id].kind.is_usage()
            && tree[id].typed_by.iter().any(|r| {
                r.target().is_some_and(|t| {
                    tree.qualified_name(t).starts_with("Library::")
                        && tree.effective_name(t) == Some(name)
                })
            })
    })
}
/// Any definition was copied from the Library into the project.
fn copied_blocks(run: &Run) -> bool {
    run.state().tree().find("Library").is_some()
}
fn searched_library(run: &Run) -> bool {
    run.library_calls.iter().any(|c| c == "search_library")
}
/// Blocks of the storage, messaging and resilience kinds were not bent to
/// fit something they are not.
fn no_unrelated_blocks(run: &Run) -> bool {
    let tree = run.state().tree();
    [
        "Library::Messaging",
        "Library::Storage",
        "Library::Resilience",
    ]
    .iter()
    .all(|package| tree.find(package).is_none())
}

/// A small service to build inside.
const THUMBNAILS: &str = r#"package Thumbnails {
    doc /* A service that makes thumbnails of uploaded images. */
    part def ThumbnailService {
        doc /* Accepts image uploads and makes their thumbnails. */
    }
}"#;

/// A shop with an empty system.
const SHOP: &str = r#"package Shop {
    doc /* An online shop. */
    part def System;
}"#;

pub fn all() -> Vec<Task> {
    vec![
        // A1–A2: building from an idea.
        Task {
            id: "a2-build-basic",
            start: EMPTY,
            locked: &[],
            messages: &[
                "I want a URL shortener: an HTTP API that shortens long URLs and resolves short codes, a store for the links, and click statistics. Keep it small.",
            ],
            answers: &[("statistic", "a separate part")],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("an API part def", |r| {
                    r.has(Some(Kind::PartDef), &["api", "http", "gateway"])
                }),
                ("a store part def", |r| {
                    r.has(Some(Kind::PartDef), &["store", "storage", "repository"])
                }),
                ("statistics modelled", |r| {
                    r.has(None, &["statistic", "clickstat", "click", "analytic"])
                }),
                ("ports or interfaces defined", |r| {
                    r.count(Kind::PortDef) + r.count(Kind::InterfaceDef) > 0
                }),
                ("the model has no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a2-build-with-requirements",
            start: EMPTY,
            locked: &[],
            messages: &[
                "Model a URL shortener with an HTTP API and a link store. Add a requirement that every short code maps to exactly one long URL, and say which part satisfies it.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a requirement exists", |r| {
                    r.count(Kind::RequirementDef) + r.count(Kind::Requirement) > 0
                }),
                ("the requirement is satisfied by a part", |r| {
                    r.count(Kind::Satisfy) > 0
                }),
                ("a store part def", |r| {
                    r.has(Some(Kind::PartDef), &["store", "storage", "repository"])
                }),
                ("the model has no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a2-ask-statistics",
            start: CORE,
            locked: &[],
            messages: &["We also need click statistics."],
            answers: &[
                ("separate", "a separate part"),
                ("statistic", "a separate part"),
            ],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("asked before deciding where statistics live", asked),
                ("statistics modelled", |r| {
                    r.has(None, &["statistic", "clickstat", "click", "analytic"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a2-ask-authentication",
            start: CORE,
            locked: &[],
            messages: &["The API needs authentication."],
            answers: &[
                ("key", "API keys"),
                ("oauth", "API keys"),
                ("how", "API keys"),
                ("which", "API keys"),
            ],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("asked how to authenticate", asked),
                ("authentication modelled", |r| {
                    r.has(None, &["auth", "key", "credential"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        // A3: adjusting in words.
        Task {
            id: "a3-connect",
            start: UNWIRED,
            locked: &[],
            messages: &["Connect the API to the store in UrlShortenerService."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a connection or interface in the service", |r| {
                    r.count(Kind::Connection) + r.count(Kind::Interface) > 0
                }),
                ("the model has no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-rename",
            start: FULL,
            locked: &[],
            messages: &["Rename LinkStore to LinkRepository."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("LinkRepository exists", |r| {
                    r.has(Some(Kind::PartDef), &["linkrepository"])
                }),
                ("LinkStore is gone", |r| {
                    !r.state()
                        .tree()
                        .walk()
                        .into_iter()
                        .any(|id| r.state().tree()[id].name.as_deref() == Some("LinkStore"))
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-attribute",
            start: FULL,
            locked: &[],
            messages: &["A short link should record when it was created."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a creation-time attribute", |r| {
                    r.has(
                        Some(Kind::Attribute),
                        &["created", "creation", "timestamp", "createdat"],
                    )
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-delete",
            start: FULL,
            locked: &[],
            messages: &["Remove the click statistics completely; we don't need them."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("click statistics are gone completely", |r| {
                    !r.has(
                        None,
                        &["clickstat", "clickreport", "clickevent", "statsport"],
                    )
                }),
                ("the model has no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-fix-problems",
            start: BROKEN,
            locked: &[],
            messages: &["Fix the problems in the model."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![("the model has no problems", no_problems)],
            follow_up: true,
        },
        Task {
            id: "a3-multiplicity",
            start: FULL,
            locked: &[],
            messages: &["The service should be able to run with up to three link stores."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a multiplicity allows three stores", |r| {
                    r.text().contains("[1..3]")
                        || r.text().contains("[0..3]")
                        || r.text().contains("[3]")
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-doc-only",
            start: FULL,
            locked: &[],
            messages: &["Add a short description to ClickStats saying it keeps counts in memory."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("the doc mentions memory", |r| {
                    r.text().to_lowercase().contains("memory")
                }),
                ("nothing else changed (one change)", |r| r.applied == 1),
            ],
            follow_up: true,
        },
        // Answers without changes.
        Task {
            id: "a3-explain",
            start: FULL,
            locked: &[],
            messages: &["What does the link store do, and what is connected to it?"],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![("nothing changed", unchanged)],
            follow_up: false,
        },
        Task {
            id: "a3-show-architecture",
            start: FULL,
            locked: &[],
            messages: &["Show me the whole architecture."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![("nothing changed", unchanged)],
            follow_up: false,
        },
        // A4: locks.
        Task {
            id: "a4-lock-refused",
            start: FULL,
            locked: &["UrlShortener::HttpApi"],
            messages: &["Rename the API's port `storage` to `links`."],
            answers: &[],
            lock_policy: LockPolicy::Refuse,
            checks: vec![
                ("the lock confirmation was asked", |r| {
                    !r.lock_prompts.is_empty()
                }),
                ("the port keeps its name", |r| {
                    r.state()
                        .tree()
                        .find("UrlShortener::HttpApi::storage")
                        .is_some()
                }),
                ("says the change was not made", |r| {
                    let reply = r.final_reply().to_lowercase().replace('\u{2019}', "'");
                    let words: Vec<&str> = reply
                        .split(|c: char| !c.is_alphanumeric() && c != '\'')
                        .collect();
                    [
                        "not",
                        "wasn't",
                        "didn't",
                        "refused",
                        "declined",
                        "unchanged",
                        "kept",
                    ]
                    .iter()
                    .any(|word| words.contains(word))
                }),
            ],
            follow_up: true,
        },
        Task {
            id: "a4-lock-allowed",
            start: FULL,
            locked: &["UrlShortener::HttpApi"],
            messages: &["Rename the API's port `storage` to `links`."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("the lock confirmation was asked", |r| {
                    !r.lock_prompts.is_empty()
                }),
                ("the port is renamed", |r| {
                    r.state()
                        .tree()
                        .find("UrlShortener::HttpApi::links")
                        .is_some()
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a4-work-around-a-lock",
            start: FULL,
            locked: &["UrlShortener::LinkStore"],
            messages: &["Add a cache in front of the link store so repeated lookups are fast."],
            answers: &[("cache", "a separate part")],
            lock_policy: LockPolicy::Refuse,
            checks: vec![
                ("a cache is modelled", |r| r.has(None, &["cache"])),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        // A8: a loosely worded new idea.
        Task {
            id: "a8-expiring-links",
            start: FULL,
            locked: &[],
            messages: &["Add expiring links."],
            answers: &[(
                "expir",
                "links expire after a duration set when they are created",
            )],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("expiry is modelled", |r| {
                    r.has(None, &["expir", "ttl", "validuntil"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a8-expiring-links-locked-store",
            start: FULL,
            locked: &["UrlShortener::LinkStore", "UrlShortener::SqlLinkStore"],
            messages: &["Add expiring links."],
            answers: &[(
                "expir",
                "links expire after a duration set when they are created",
            )],
            lock_policy: LockPolicy::Refuse,
            checks: vec![
                (
                    "expiry is modelled on unlocked elements, or the Assistant said why not",
                    |r| r.has(None, &["expir", "ttl", "validuntil"]) || !r.lock_prompts.is_empty(),
                ),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a8-vague-robustness",
            start: FULL,
            locked: &[],
            messages: &["Make it more robust."],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![("asked what robust means before changing much", |r| {
                r.asked() || r.applied <= 1
            })],
            follow_up: true,
        },
        Task {
            id: "a8-rate-limiting",
            start: FULL,
            locked: &[],
            messages: &["Add rate limiting for shorten requests."],
            answers: &[
                ("where", "in the API"),
                ("separate", "part of the API"),
                ("limit", "100 per minute per client"),
            ],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("rate limiting is modelled", |r| {
                    r.has(None, &["rate", "limit", "throttl"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        // Simplicity and names (C-10).
        Task {
            id: "a3-plain-names",
            start: CORE,
            locked: &[],
            messages: &["When a link is created, the owner should get an email."],
            answers: &[("email", "a separate part")],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a plainly named notification part", |r| {
                    r.has(Some(Kind::PartDef), &["notif", "email", "mail"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-general-not-duplicated",
            start: CORE,
            locked: &[],
            messages: &[
                "Links can point to web pages, images or documents; the shortener treats them all the same except that it records which kind it is.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("no near-duplicate link definitions", |r| {
                    let tree = r.state().tree();
                    tree.walk()
                        .into_iter()
                        .filter(|id| matches!(tree[*id].kind, Kind::ItemDef | Kind::PartDef))
                        .filter(|id| {
                            tree[*id].name.as_deref().is_some_and(|n| {
                                n.to_lowercase().contains("link") && n != "ShortLink"
                            })
                        })
                        .count()
                        <= 2
                }),
                ("the kind is recorded", |r| {
                    r.has(None, &["kind", "type", "category"])
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a3-two-steps",
            start: CORE,
            locked: &[],
            messages: &[
                "Add a health check endpoint to the API.",
                "Now make the store report its health too.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("health modelled", |r| r.has(None, &["health"])),
                ("changes applied", changed),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "a1-empty-question",
            start: EMPTY,
            locked: &[],
            messages: &["What can you help me with?"],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![("nothing changed", unchanged)],
            follow_up: false,
        },
        // H8: reusing building blocks when they fit, and only then (C-49).
        Task {
            id: "h8-reuse-async-worker",
            start: THUMBNAILS,
            locked: &[],
            messages: &[
                "Uploads should return at once: put thumbnail jobs on a queue and have a background worker make the thumbnails, with failed jobs tried again. Model that inside ThumbnailService.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("searched the Library", searched_library),
                (
                    "reused the asynchronous worker, or a queue and a worker",
                    |r| {
                        uses_block(r, "AsyncWorker")
                            || (uses_block(r, "Queue") && uses_block(r, "Worker"))
                    },
                ),
                ("no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "h8-reuse-rate-limit-with-a-value",
            start: SHOP,
            locked: &[],
            messages: &[
                "Give System a public API that lets each client make at most 100 requests per minute, in front of a backend service that handles the requests.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("searched the Library", searched_library),
                ("reused the rate limiter", |r| {
                    uses_block(r, "RateLimitedApi") || uses_block(r, "RateLimiter")
                }),
                ("the limit is 100", |r| {
                    r.text().contains("limitPerMinute = 100")
                }),
                ("no problems", no_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "h8-no-block-fits",
            start: FULL,
            locked: &[],
            messages: &[
                "Add a QR code generator that turns a short link into a QR code image, used by the API.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("a QR code generator in the project", |r| {
                    let tree = r.state().tree();
                    tree.walk().into_iter().any(|id| {
                        tree[id].kind == Kind::PartDef
                            && !tree.qualified_name(id).starts_with("Library::")
                            && tree
                                .effective_name(id)
                                .is_some_and(|n| n.to_lowercase().contains("qr"))
                    })
                }),
                ("no unrelated block bent to fit", no_unrelated_blocks),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "h8-project-concept-first",
            start: CORE,
            locked: &[],
            messages: &[
                "Store the links in a relational database with an index on the short code.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("the project's link store is kept", |r| {
                    r.state().tree().find("UrlShortener::LinkStore").is_some()
                }),
                ("no Library block copied for it", |r| !copied_blocks(r)),
                ("the database is modelled", |r| {
                    r.has(None, &["sql", "relational", "database"])
                        || r.text().to_lowercase().contains("relational")
                }),
                ("no new problems", no_new_problems),
            ],
            follow_up: true,
        },
        Task {
            id: "h8-save-when-asked",
            start: FULL,
            locked: &[],
            messages: &[
                "Save the LinkStore definition to My Library so I can use it in my other projects.",
            ],
            answers: &[],
            lock_policy: LockPolicy::Allow,
            checks: vec![
                ("saved once", |r| r.saves == 1),
                ("the model is unchanged", unchanged),
            ],
            follow_up: false,
        },
    ]
}
