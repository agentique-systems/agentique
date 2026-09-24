//! Bounded decision questions, independent of any inference provider or runtime.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionState {
    pub intent: String,
    /// Revision identity of the observation, not a provider-controlled version.
    pub revision: String,
    pub selected_elements: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DecisionQuestion {
    Choice {
        id: String,
        options: Vec<String>,
    },
    Score {
        id: String,
        minimum: f64,
        maximum: f64,
    },
    Boolean {
        id: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DecisionValue {
    Choice(String),
    Score(f64),
    Boolean(bool),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionAnswer {
    pub question: String,
    pub value: DecisionValue,
    pub probabilities: Vec<(String, f64)>,
    /// Mock certainty is not calibrated model confidence.
    pub confidence: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionResult {
    pub provider: String,
    pub revision: String,
    pub answers: Vec<DecisionAnswer>,
}
#[derive(Debug, thiserror::Error)]
#[error("invalid bounded decision: {0}")]
pub struct DecisionError(pub String);

pub trait DecisionModel: Send + Sync {
    fn decide(
        &self,
        state: &DecisionState,
        questions: &[DecisionQuestion],
    ) -> Result<DecisionResult, DecisionError>;
}

/// A deterministic demonstration of intent-to-view routing. No external model call.
pub struct MockViewDecision;
impl DecisionModel for MockViewDecision {
    fn decide(
        &self,
        state: &DecisionState,
        questions: &[DecisionQuestion],
    ) -> Result<DecisionResult, DecisionError> {
        let intent = state.intent.to_lowercase();
        let preferred =
            if intent.contains("chang") || intent.contains("histor") || intent.contains("diff") {
                "History"
            } else if intent.contains("require") || intent.contains("verif") {
                "Requirements"
            } else if intent.contains("depend")
                || intent.contains("graph")
                || intent.contains("connect")
            {
                "Graph"
            } else {
                "Architecture"
            };
        let mut answers = Vec::new();
        for question in questions {
            let DecisionQuestion::Choice { id, options } = question else {
                return Err(DecisionError(
                    "demo provider supports only the view Choice question".into(),
                ));
            };
            if !options.iter().any(|option| option == preferred) {
                return Err(DecisionError(
                    "selected view is outside offered choices".into(),
                ));
            }
            answers.push(DecisionAnswer {
                question: id.clone(),
                value: DecisionValue::Choice(preferred.into()),
                probabilities: Vec::new(),
                confidence: None,
            });
        }
        Ok(DecisionResult {
            provider: "deterministic-view-demo".into(),
            revision: state.revision.clone(),
            answers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_routing_retains_revision_and_rejects_unoffered_answers() {
        let state = DecisionState {
            intent: "Show its dependencies".into(),
            revision: "revision-a".into(),
            selected_elements: vec![],
        };
        let question = DecisionQuestion::Choice {
            id: "lens".into(),
            options: vec!["Architecture".into(), "Graph".into()],
        };
        let result = MockViewDecision.decide(&state, &[question]).unwrap();
        assert_eq!(result.revision, "revision-a");
        assert!(
            matches!(&result.answers[0].value, DecisionValue::Choice(value) if value == "Graph")
        );
        assert!(result.answers[0].confidence.is_none());
        assert!(
            MockViewDecision
                .decide(
                    &state,
                    &[DecisionQuestion::Choice {
                        id: "lens".into(),
                        options: vec!["History".into()]
                    }]
                )
                .is_err()
        );
    }
}
