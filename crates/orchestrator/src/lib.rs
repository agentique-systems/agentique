//! The Orchestrator (ROADMAP §4.16, C-53; part `Orchestrator` in
//! `model/Agentique.sysml`): takes an objective the Operator gives in the
//! Studio through cycles that improve Agentique itself. Deterministic code
//! decides each phase from recorded results; agents (each its own Claude
//! Agent runtime session) propose, implement, review and evaluate; required
//! checks, gates and an independent review decide what is merged; a merged
//! change is built, tried and adopted, and the objective goes on in the
//! adopted build.
//!
//! - [`record`]: the objective's durable record and its journal of side
//!   effects, so a restart neither repeats nor loses one.
//! - [`gates`]: deterministic gates on a change (protected paths, keys,
//!   the baseline guard on tests and checks).
//! - [`forge`]: the branch, pull request, the repository's checks, the merge.
//! - [`builds`]: release builds for adoption, debug builds for test instances.
//! - [`control`]: test instances and the client of their control interface.
//! - [`roles`]: what each agent is told and which tools it has.
//! - [`run`]: the driver.
#![forbid(unsafe_code)]

pub mod builds;
pub mod control;
pub mod forge;
pub mod gates;
pub mod record;
pub mod roles;
pub mod run;
