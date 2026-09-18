//! Source-to-kernel orchestration. The canonical model has no parser dependency.
#![forbid(unsafe_code)]
mod lowering;
mod project;
pub use project::*;

pub use agq_kerml_syntax as syntax;
use agq_kernel::{DocumentId, SourceRevisionId};
pub use lowering::{
    FrontendDiagnostic, ReferenceAssertion, ValidatedModel, ValidationFailure, WorkingModel,
};
use std::{collections::BTreeMap, sync::Arc};
use syntax::{ParseLimits, SourceError, SyntaxDocument, TextEdit};

#[derive(Debug, thiserror::Error)]
pub enum FrontendError {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Kernel(#[from] agq_kernel::ModelError),
}
/// In-memory source history. Publication is atomic on successful syntax/model
/// construction; this is not an application persistence or checkpoint API.
pub struct Document {
    limits: ParseLimits,
    current: SourceRevisionId,
    history: BTreeMap<SourceRevisionId, Arc<WorkingModel>>,
}
impl Document {
    pub fn new(source: &str) -> Result<Self, FrontendError> {
        Self::with_limits(source, ParseLimits::default())
    }
    pub fn with_limits(source: &str, limits: ParseLimits) -> Result<Self, FrontendError> {
        let syntax = syntax::parse(DocumentId::new(), source, limits)?;
        let model = Arc::new(lowering::lower(syntax, None)?);
        let current = model.syntax().revision();
        Ok(Self {
            limits,
            current,
            history: BTreeMap::from([(current, model)]),
        })
    }
    pub fn current(&self) -> &Arc<WorkingModel> {
        &self.history[&self.current]
    }
    pub fn revision(&self, revision: SourceRevisionId) -> Option<&Arc<WorkingModel>> {
        self.history.get(&revision)
    }
    pub fn source(&self, revision: SourceRevisionId) -> Option<&SyntaxDocument> {
        self.revision(revision).map(|m| m.syntax())
    }
    pub fn edit(&mut self, edit: TextEdit) -> Result<Arc<WorkingModel>, FrontendError> {
        let syntax = self.current().syntax().edit(&edit, self.limits)?;
        self.publish(syntax)
    }
    /// External replacement carries no source-edit continuity evidence.
    pub fn replace(&mut self, source: &str) -> Result<Arc<WorkingModel>, FrontendError> {
        let syntax = syntax::parse(self.current().syntax().document(), source, self.limits)?;
        self.publish(syntax)
    }
    fn publish(&mut self, syntax: SyntaxDocument) -> Result<Arc<WorkingModel>, FrontendError> {
        let model = Arc::new(lowering::lower(syntax, Some(self.current()))?);
        self.current = model.syntax().revision();
        self.history.insert(self.current, model.clone());
        Ok(model)
    }
}
