//! The Assistant (REALIGNMENT §3.1, §3.2): the AI agent the Operator works
//! with in the Conversation.
//!
//! - [`tools`]: what the Assistant can do. Tools read the System State or turn
//!   a request into one System State change, which the Studio applies like
//!   any Operator edit, so locks and undo work the same for both.
//! - [`turn`]: the tool-use loop for one turn of the Conversation, directly
//!   ([`turn::run`]) or on a background thread ([`BackgroundTurn`]).
//! - [`claude`]: the Claude API client.
//! - [`skills`]: the system prompt, compiled in from `skills/*.md`.
//! - [`conversation`]: the per-project conversation, stored as the Claude API
//!   content blocks it consists of.
//! - [`model`]: the language model interface, with a scripted stand-in for
//!   tests.
#![forbid(unsafe_code)]

pub mod claude;
pub mod conversation;
pub mod model;
pub mod skills;
pub mod tools;
pub mod turn;

pub use claude::ClaudeModel;
pub use conversation::{ChangeSummary, Conversation, Entry, ToolResult};
pub use model::{Model, ModelError, Reply, Request, ScriptedModel, StreamEvent, Usage};
pub use skills::system_prompt;
pub use tools::Prepared;
pub use turn::{BackgroundEvent, BackgroundTurn, ToolCall, TurnEvent};
