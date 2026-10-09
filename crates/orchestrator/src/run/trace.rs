//! Traceability at review (C-55, ROADMAP §4.16): what the reviewed commit
//! changed against what its proposal named, and the cumulative change since
//! the Operator's approved baseline, recorded on the cycle, each said in
//! one thread entry, and given to the reviewer to judge
//! (`crate::traceability` computes them).

use super::Driver;
use crate::record::Proposal;
use crate::thread::ThreadEntry;
use crate::traceability;
use agq_execution::git::Patch;
use std::path::Path;

impl Driver {
    /// Traceability and the cumulative change of the reviewed `commit`
    /// (checked out clean in `verify`; `patch` against `base`), since the
    /// approved baseline or, without one, the objective's start (its first
    /// cycle's base): recorded on the cycle, said in the thread, and
    /// returned as the reviewer reads them.
    pub(super) fn trace_for_review(
        &mut self,
        verify: &Path,
        base: &str,
        commit: &str,
        patch: &Patch,
        proposal: &Proposal,
    ) -> String {
        let repository = self.objective.repository.clone();
        let traced = traceability::traced_in(&repository, verify, base, commit, patch, proposal);
        let start = self
            .objective
            .cycles
            .first()
            .and_then(|c| c.base.clone())
            .unwrap_or_else(|| base.to_string());
        let cumulative = traceability::cumulative_in(&repository, verify, &start, commit);
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
        text
    }
}
