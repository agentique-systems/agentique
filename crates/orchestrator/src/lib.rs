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
//!   effects, so a restart neither repeats nor loses one; its agents'
//!   directives.
//! - [`thread`]: the objective's thread, what happened in time order as the
//!   Conversation and the Objectives panel show it (C-54).
//! - [`gates`]: deterministic gates on a change (protected paths, keys,
//!   the baseline guard on tests and checks).
//! - [`forge`]: the branch, pull request, the repository's checks, the merge.
//! - [`builds`]: release builds for adoption, debug builds for test instances.
//! - [`control`]: test instances and the client of their control interface.
//! - [`roles`]: what each agent is told and which tools it has.
//! - [`decide`]: typed decisions in operation (System 1) with escalation.
//! - [`explore`]: exploration of a test instance toward a goal (C-54).
//! - [`findings`]: the checks exploration makes, and findings reproduced
//!   and reduced by replaying them.
//! - [`knowledge`]: the testing knowledge kept across runs.
//! - [`observed`]: what exploration reads from a test instance, in one
//!   place.
//! - [`models`]: each role's model, resolved before an objective starts
//!   (C-54).
//! - [`run`]: the driver; its modules run exploring cycles, evidence on
//!   the base and delegated child objectives (C-54).
#![forbid(unsafe_code)]

pub mod builds;
pub mod control;
pub mod decide;
pub mod explore;
pub mod findings;
pub mod forge;
pub mod gates;
pub mod knowledge;
pub mod models;
pub mod observed;
pub mod record;
pub mod roles;
pub mod run;
pub mod thread;
