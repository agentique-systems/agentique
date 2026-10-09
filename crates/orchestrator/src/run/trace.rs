//! Traceability in the driver (C-55, ROADMAP §4.16): the clean checkout the
//! model is read through for the gates and the review (never `verify`,
//! where the checks ran the change's code); the commits whose purpose
//! requirement the objective protects; and, at review, what the reviewed
//! commit changed against what its proposal named and the cumulative
//! change since the Operator's approved baseline, recorded on the cycle,
//! each said in one thread entry, and given to the reviewer to judge
//! (`crate::traceability` computes them).

use super::Driver;
use crate::record::Proposal;
use crate::thread::ThreadEntry;
use crate::traceability;
use agq_execution::git::{self, Patch};
use std::path::PathBuf;

impl Driver {
    /// A clean checkout, made now, of the first of `commits` whose tree has
    /// a model, in which nothing runs: what the gates and traceability read
    /// models through (by commit, through History, so any commit of the
    /// repository can be read in it). None when none of them has a model.
    pub(super) fn reader(&mut self, commits: &[&str]) -> Result<Option<PathBuf>, String> {
        let repository = self.objective.repository.clone();
        for commit in commits {
            if traceability::has_model(&repository, commit)? {
                let name = "model";
                let worktree = format!("{}-{}-{name}", self.id(), self.cycle().n);
                let _ = git::remove_worktree(&repository, &worktree);
                crate::control::remove_folder(&self.folder(name));
                return self.checkout(name, commit).map(Some);
            }
        }
        Ok(None)
    }

    /// The commits whose purpose requirement the objective protects beside
    /// each cycle's base (C-55): its start (its first cycle's base) and the
    /// approved baseline on the remote recorded when it was created, so that
    /// moving the purpose out of a root package in one cycle hides it from
    /// no later one.
    pub(super) fn protected_commits(&self) -> Vec<String> {
        let mut commits = Vec::new();
        if let Some(start) = self.objective.cycles.first().and_then(|c| c.base.clone()) {
            let (since, approved, _) = traceability::since(
                &self.objective.repository,
                self.objective.origin.as_deref(),
                &start,
            );
            commits.push(start);
            if approved {
                commits.push(since);
            }
        }
        commits
    }

    /// Traceability and the cumulative change of the reviewed `commit`
    /// (`patch` against `base`), since the approved baseline or, without
    /// one, the objective's start (its first cycle's base): recorded on the
    /// cycle, said in the thread, and returned as the reviewer reads them.
    pub(super) fn trace_for_review(
        &mut self,
        base: &str,
        commit: &str,
        patch: &Patch,
        proposal: &Proposal,
    ) -> Result<String, String> {
        let repository = self.objective.repository.clone();
        let reader = self.reader(&[commit, base])?;
        let traced = traceability::traced_in(
            &repository,
            reader.as_deref(),
            base,
            commit,
            patch,
            proposal,
        );
        let start = self
            .objective
            .cycles
            .first()
            .and_then(|c| c.base.clone())
            .unwrap_or_else(|| base.to_string());
        let cumulative = traceability::cumulative_in(
            &repository,
            reader.as_deref(),
            self.objective.origin.as_deref(),
            &start,
            commit,
        );
        self.post(ThreadEntry::event(traced.line()).with_details(traced.text()));
        self.post(ThreadEntry::event(cumulative.line()).with_details(cumulative.text()));
        let text = format!(
            "Traceability: what the commit changed against what the proposal named in `parts` (judge each entry explicitly, as you judge test changes; naming an element names what it owns):\n{}\n\n{}",
            traced.text(),
            cumulative.text()
        );
        let cycle = self.cycle_mut();
        cycle.traceability = Some(traced);
        cycle.cumulative = Some(cumulative);
        self.save();
        Ok(text)
    }
}
