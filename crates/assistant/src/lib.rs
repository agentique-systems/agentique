//! The Assistant (ROADMAP §4.1, §4.10): the AI agent the Operator works
//! with in the Conversation.
//!
//! - [`tools`]: what the Assistant can do. Tools read the System State or turn
//!   a request into one System State change, which the Studio applies like
//!   any Operator edit, so locks and undo work the same for both.
//! - [`turn`]: the tool-use loop for one turn of the Conversation, directly
//!   ([`turn::run`]) or on a background thread ([`BackgroundTurn`]).
//! - [`provider_model`]: the model through the provider layer
//!   (`agq-providers`, on rig), and [`choice`]: which model to use.
//! - [`claude`]: the hand-written Claude API client (until W5.7).
//! - [`skills`]: the system prompt, compiled in from `skills/*.md`.
//! - [`conversation`]: the per-project conversation, stored as the Claude API
//!   content blocks it consists of.
//! - [`model`]: the language model interface, with a scripted stand-in for
//!   tests.
//! - [`worker`]: an implementation worker, the same loop with code tools
//!   in one worktree (C-50).
//! - [`runtime`]: what runs a turn ([`Runtime`]): the loop above over a
//!   model ([`LoopRuntime`]), or [`claude_agent`]: the Claude Agent SDK's
//!   loop in a companion process (C-51), with Agentique's tools only or, in
//!   a development session, the SDK's tools under a [`policy`] (C-53).
#![forbid(unsafe_code)]

pub mod choice;
pub mod claude;
pub mod claude_agent;
pub mod conversation;
pub mod model;
pub mod policy;
pub mod provider_model;
pub mod runtime;
pub mod skills;
pub mod sysml_text;
pub mod tools;
pub use tools::phase;
pub mod turn;
pub mod worker;

pub use choice::ModelChoice;
pub use claude::ClaudeModel;
pub use conversation::{ChangeSummary, Conversation, Entry, ToolResult};
pub use model::{Model, ModelError, Reply, Request, ScriptedModel, StreamEvent, Usage};
pub use provider_model::ProviderModel;
pub use runtime::{LoopRuntime, Runtime};
pub use skills::system_prompt;
pub use tools::Prepared;
pub use turn::{Activity, BackgroundEvent, BackgroundTurn, TaskEvent, ToolCall, TurnEvent};
