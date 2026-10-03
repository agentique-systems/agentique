//! Implementation (ROADMAP §4.15; part `Implementation` in
//! `model/Agentique.sysml`): what the model says about code.
//!
//! - [`links`]: implementation links in `model/links.json`, by element
//!   identity, many to many; model to code and code to model.
//! - [`checks`]: the supported checks, each with its coverage: dependency
//!   boundaries (linked modules, or crates against the self-model), mapped
//!   contract shapes, linked tests; failing checks are drift.
//! - [`harness`]: scenarios run against the real code through the
//!   project's harness, with the same step interpreter and checks as model
//!   execution.
//! - [`rust`]: just enough reading of Rust source for those checks.
//! - [`task`]: an implementation task's brief, from the model, its required
//!   checks, and the verification of a working copy the worker and the
//!   Studio both run.
//! - [`responsibility`]: a part as the Operator and the Assistant read it
//!   (purpose, what it owns, contract, dependencies, code, checks, impact).
//! - [`CheckReport`]: a set of check results with the code and model they
//!   saw, kept with the run results in the app's data.
//!
//! Processes run through Execution; nothing here writes code.
#![forbid(unsafe_code)]

pub mod checks;
pub mod harness;
pub mod links;
pub mod responsibility;
pub mod rust;
pub mod task;

pub use checks::{CheckKind, ImplementationCheck, drift};
pub use harness::{HarnessTarget, run_implementation};
pub use links::{Link, LinkKind, Links};

use agq_execution::git;
use agq_language::Tree;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The results of one round of implementation checks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub format: u32,
    pub id: String,
    pub started: String,
    pub repository: String,
    pub commit: String,
    pub dirty: bool,
    pub tree_digest: String,
    /// Digest of the model slice the links reach.
    pub model_digest: String,
    pub checks: Vec<ImplementationCheck>,
}

impl CheckReport {
    /// A report of `checks` on the code in `repository` as it is now.
    pub fn new(
        tree: &Tree,
        links: &Links,
        repository: &Path,
        checks: Vec<ImplementationCheck>,
    ) -> CheckReport {
        CheckReport {
            format: 1,
            id: agq_simulation::new_run_id(),
            started: agq_simulation::result::utc_now(),
            repository: repository.to_string_lossy().into_owned(),
            commit: git::head(repository)
                .map(|h| h.commit)
                .unwrap_or_else(|_| "unknown".into()),
            dirty: git::changed_files(repository).is_ok_and(|c| !c.is_empty()),
            tree_digest: git::tree_digest(repository).unwrap_or_default(),
            model_digest: links_digest(tree, links),
            checks,
        }
    }

    /// Whether the report still describes the model and the code.
    pub fn freshness(
        &self,
        tree: &Tree,
        links: &Links,
        repository: &Path,
    ) -> agq_simulation::Freshness {
        if links_digest(tree, links) != self.model_digest {
            return agq_simulation::Freshness::Outdated(
                "the linked model elements changed since these checks ran".into(),
            );
        }
        if git::tree_digest(repository).unwrap_or_default() != self.tree_digest {
            return agq_simulation::Freshness::Outdated(
                "the code changed since these checks ran".into(),
            );
        }
        agq_simulation::Freshness::Current
    }
}

/// The digest of the model slice every linked element reaches, with the links.
pub fn links_digest(tree: &Tree, links: &Links) -> String {
    let mut text = links.to_text();
    let mut elements: Vec<_> = links.links.iter().map(|l| l.element()).collect();
    elements.sort();
    elements.dedup();
    for element in elements {
        if tree.contains(element) {
            text.push_str(&agq_simulation::digest::model_digest(tree, element));
        }
    }
    agq_simulation::digest::text_digest(&text)
}
