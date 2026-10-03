//! What runs the Assistant's turns (ROADMAP §4.10, C-51): a [`Runtime`].
//!
//! - [`LoopRuntime`]: Agentique's own loop ([`crate::turn::run_with`]) over a
//!   [`Model`], for every provider.
//! - [`crate::claude_agent::ClaudeAgent`]: the Claude Agent SDK's loop in a
//!   companion process; Agentique only carries out its tool calls.
//!
//! Both take the same conversation, tools, bound, executor, event sink and
//! stop flag, so the Studio's tool executor, approvals, locks and
//! conversation format are the same whichever runs the turn. What differs is
//! said, not hidden: the Claude Agent runtime is Anthropic's only and keeps
//! its own context.

use crate::conversation::{Conversation, ToolResult};
use crate::model::Model;
use crate::turn::{ToolCall, Toolset, TurnEvent, run_with};
use agq_providers::ModelRef;
use std::sync::atomic::AtomicBool;

pub trait Runtime: Send {
    /// The model behind it when known before the turn: tags its replies and
    /// prices their usage.
    fn model(&self) -> Option<ModelRef>;

    /// What runs the turn, for the Conversation's header.
    fn label(&self) -> String;

    /// Runs one turn: answers the conversation, whose last entry is the
    /// Operator's message, adding every entry it makes to `conversation` and
    /// reporting it as [`TurnEvent::Entry`]. Every tool call goes to
    /// `execute` (which checks its input, [`checked`]), one at a time; every
    /// [`crate::StreamEvent::ToolCallStarted`] is followed by one
    /// [`TurnEvent::ToolFinished`]. At most `max_calls` model calls. Ends
    /// promptly once `stop` is set, with a notice; the conversation is left
    /// well formed for the next turn.
    fn run(
        &mut self,
        conversation: &mut Conversation,
        toolset: &Toolset,
        max_calls: usize,
        execute: &mut dyn FnMut(&ToolCall) -> ToolResult,
        on_event: &mut dyn FnMut(TurnEvent),
        stop: &AtomicBool,
    );
}

/// Agentique's own loop over one model (R-21): the model is called once per
/// step and the loop decides what happens with its reply.
pub struct LoopRuntime(pub Box<dyn Model + Send>);

impl LoopRuntime {
    pub fn boxed(model: Box<dyn Model + Send>) -> Box<dyn Runtime> {
        Box::new(LoopRuntime(model))
    }
}

impl Runtime for LoopRuntime {
    fn model(&self) -> Option<ModelRef> {
        self.0.model()
    }

    fn label(&self) -> String {
        match self.0.model() {
            Some(model) => model.model,
            None => "a scripted model".into(),
        }
    }

    fn run(
        &mut self,
        conversation: &mut Conversation,
        toolset: &Toolset,
        max_calls: usize,
        execute: &mut dyn FnMut(&ToolCall) -> ToolResult,
        on_event: &mut dyn FnMut(TurnEvent),
        stop: &AtomicBool,
    ) {
        run_with(
            self.0.as_mut(),
            conversation,
            toolset,
            max_calls,
            execute,
            on_event,
            stop,
        )
    }
}

/// `execute` behind the input check every tool call passes, whatever runtime
/// made it: a call to a tool the toolset does not have, or whose input does
/// not match the tool's schema, is answered with an error and never carried
/// out (ROADMAP §4.2: the Assistant's output is untrusted input).
pub fn checked<'a>(
    definitions: &'a serde_json::Value,
    execute: &'a mut dyn FnMut(&ToolCall) -> ToolResult,
) -> impl FnMut(&ToolCall) -> ToolResult + 'a {
    move |call: &ToolCall| match crate::tools::check_input_against(
        definitions,
        &call.name,
        &call.input,
    ) {
        Ok(()) => execute(call),
        Err(message) => ToolResult::error(format!("Not run: {message}.")),
    }
}
