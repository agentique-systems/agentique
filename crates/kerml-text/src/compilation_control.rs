//! Optional operational control for unpublished source compilation.
use agq_kerml_semantics::{CancellationToken, Cancelled};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

/// Last actually entered operation. Refinement can revisit earlier stages.
/// Effective validation is the source audit, not a platform Validated handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CompilationStage {
    Queued,
    Parsing,
    DeclaredModel,
    Resolving,
    SemanticClosure,
    EffectiveValidation,
    PreparingReview,
}

#[derive(Debug)]
struct Control {
    cancellation: CancellationToken,
    stage: AtomicU8,
}

/// A source operation's explicit control. The default is disabled, preserving
/// ordinary uncancelled call paths. This is never serialized or semantic input.
#[derive(Clone, Debug, Default)]
pub struct CompilationControl(Option<Arc<Control>>);

impl CompilationControl {
    /// Enable cancellation and bounded last-stage observation for a new task.
    pub fn new() -> Self {
        Self(Some(Arc::new(Control {
            cancellation: CancellationToken::default(),
            stage: AtomicU8::new(CompilationStage::Queued as u8),
        })))
    }

    /// Request stopping; a terminal worker outcome must still acknowledge it.
    pub fn cancel(&self) {
        if let Some(control) = &self.0 {
            control.cancellation.request();
        }
    }

    /// Test the request at a safe boundary without changing the stage.
    pub fn check(&self) -> Result<(), Cancelled> {
        self.0
            .as_ref()
            .map_or(Ok(()), |control| control.cancellation.check())
    }

    /// Enter a real stage after checking for cancellation.
    pub fn enter(&self, stage: CompilationStage) -> Result<(), Cancelled> {
        self.check()?;
        if let Some(control) = &self.0 {
            control.stage.store(stage as u8, Ordering::Release);
        }
        Ok(())
    }

    /// Share only this task's cancellation request with the producer scheduler.
    pub fn cancellation(&self) -> Option<CancellationToken> {
        self.0.as_ref().map(|control| control.cancellation.clone())
    }

    /// Last entered stage, or None for the ordinary disabled control.
    pub fn stage(&self) -> Option<CompilationStage> {
        self.0
            .as_ref()
            .map(|control| match control.stage.load(Ordering::Acquire) {
                0 => CompilationStage::Queued,
                1 => CompilationStage::Parsing,
                2 => CompilationStage::DeclaredModel,
                3 => CompilationStage::Resolving,
                4 => CompilationStage::SemanticClosure,
                5 => CompilationStage::EffectiveValidation,
                6 => CompilationStage::PreparingReview,
                _ => unreachable!("stage written only from CompilationStage"),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_control_stays_disabled_and_cancel_does_not_invent_progress() {
        let disabled = CompilationControl::default();
        disabled.cancel();
        assert!(disabled.enter(CompilationStage::Parsing).is_ok());
        assert!(disabled.stage().is_none());
        assert!(disabled.cancellation().is_none());
        let task = CompilationControl::new();
        let worker = task.clone();
        worker.enter(CompilationStage::Parsing).unwrap();
        task.cancel();
        assert_eq!(
            worker.enter(CompilationStage::DeclaredModel),
            Err(Cancelled)
        );
        assert_eq!(task.stage(), Some(CompilationStage::Parsing));
        assert!(CompilationControl::new().check().is_ok());
    }
}
