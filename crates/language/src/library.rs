//! The built-in library: the few standard library elements the subset needs,
//! with the pinned libraries' qualified names, and Agentique's own `Agents`
//! and `Scenarios` libraries (C-42, C-50). Parsed once, shared, immutable.
//! See docs/deviations.md: the pinned `ScalarValues` declares KerML
//! `datatype`s; here they are `attribute def`s, the SysML equivalent
//! (deviation 1); `Agents` and `Scenarios` are not part of the SysML standard
//! library (deviations 12 and 13).

use crate::parser::{Source, parse_into};
use crate::tree::{FIRST_LIBRARY_ID, Tree};
use std::sync::OnceLock;

pub const LIBRARY_TEXT: &str = "\
package ScalarValues {
    abstract attribute def ScalarValue;
    attribute def Boolean :> ScalarValue;
    attribute def String :> ScalarValue;
    abstract attribute def NumericalValue :> ScalarValue;
    abstract attribute def Number :> NumericalValue;
    attribute def Complex :> Number;
    attribute def Real :> Complex;
    attribute def Rational :> Real;
    attribute def Integer :> Rational;
    attribute def Natural :> Integer;
    attribute def Positive :> Natural;
}

package Agents {
    doc /* Agentique's built-in library for agents (C-42). Not part of the
         * SysML standard library (deviation 12). */
    enum def AgentMode {
        doc /* fast: quick, cheap decisions (classify, route, score, extract).
             * deliberate: reasoning (plan, use tools, handle hard cases). */
        enum fast;
        enum deliberate;
    }
    abstract item def AgentOutput {
        doc /* An answer of an agent's model, with the model's own confidence
             * between 0 and 1. The confidence is the model's claim, not a
             * measured reliability. */
        attribute confidence : ScalarValues::Real [0..1];
    }
    abstract part def Agent {
        doc /* A part whose behaviour is produced by an AI model. Its inputs and
             * outputs are ports; the tools it may use are ports connected to the
             * parts that provide them; its guardrails are requirements whose
             * subject is the agent or its contract. model is the provider's
             * model id. When the model fails (no answer within maxLatencyMs, an
             * answer that breaks the contract, a refusal, a tool it cannot
             * reach) or answers below minConfidence, the call goes to fallback:
             * a deterministic part that shares the agent's contract. Without a
             * fallback the failure stands and no outcome is guessed. */
        attribute mode : AgentMode;
        attribute model : ScalarValues::String [0..1];
        attribute minConfidence : ScalarValues::Real [0..1];
        attribute maxLatencyMs : ScalarValues::Natural [0..1];
        attribute maxCostPerCallUsd : ScalarValues::Real [0..1];
        part fallback [0..1];
    }
}

package Scenarios {
    doc /* Agentique's built-in definitions for scenarios (C-50). Not part of
         * the SysML standard library (deviation 13). A scenario is a
         * verification def: its subject is the system under examination, its
         * objective names the requirements it verifies, its steps send inputs
         * and accept outputs through the subject's ports, and its assert
         * constraints are its checks. */
    enum def Outcome {
        doc /* How a stand-in answers one call: with its output (answer), not
             * at all (timeout), or, for an agent's model, with an answer that
             * breaks the contract (invalidOutput), a refusal (refusal) or a
             * tool it cannot reach (toolUnavailable). */
        enum answer;
        enum timeout;
        enum invalidOutput;
        enum refusal;
        enum toolUnavailable;
    }
    part def StandIn {
        doc /* Answers the calls to one part of the subject in place of its own
             * behaviour, or in place of an agent's model: target is that part,
             * call the call it answers (every call when not given), outcome how
             * it answers, output what it answers with, and latencyMs how long
             * it takes in logical time. */
        ref target;
        attribute call : ScalarValues::Positive [0..1];
        attribute outcome : Outcome;
        attribute latencyMs : ScalarValues::Natural [0..1];
        ref output [0..1];
    }
}
";

/// The shared library tree.
pub fn library() -> &'static Tree {
    static LIBRARY: OnceLock<Tree> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let mut tree = Tree::with_first_id(FIRST_LIBRARY_ID);
        parse_into(
            &mut tree,
            &[Source::new("<built-in library>", LIBRARY_TEXT)],
        );
        tree
    })
}
