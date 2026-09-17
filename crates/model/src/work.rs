//! Adapter-independent cooperative work budget. Cancellation stops before commit.
use crate::{Error, Result};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub stage: String,
    pub completed: usize,
    pub total: usize,
    pub cancellation_requested: bool,
    pub committing: bool,
}
#[derive(Clone)]
pub struct WorkControl(Arc<Mutex<Progress>>);
impl Default for WorkControl {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Progress {
            stage: "queued".into(),
            completed: 0,
            total: 0,
            cancellation_requested: false,
            committing: false,
        })))
    }
}
impl WorkControl {
    pub fn progress(&self) -> Progress {
        self.0.lock().unwrap().clone()
    }
    /// False means the atomic durable commit has already started; inspect the result.
    pub fn cancel(&self) -> bool {
        let mut p = self.0.lock().unwrap();
        if p.committing {
            return false;
        }
        p.cancellation_requested = true;
        true
    }
    pub fn check(&self) -> Result<()> {
        if self.0.lock().unwrap().cancellation_requested {
            Err(Error::new(
                "cancelled",
                "Work cancelled before durable commit",
            ))
        } else {
            Ok(())
        }
    }
    pub fn update(&self, stage: &str, completed: usize, total: usize) -> Result<()> {
        self.check()?;
        let mut p = self.0.lock().unwrap();
        p.stage = stage.into();
        p.completed = completed;
        p.total = total;
        Ok(())
    }
    /// Serializes the cancellation/commit race. No cancellation is acknowledged after this point.
    pub fn begin_commit(&self) -> Result<()> {
        let mut p = self.0.lock().unwrap();
        if p.cancellation_requested {
            return Err(Error::new(
                "cancelled",
                "Work cancelled before durable commit",
            ));
        }
        p.committing = true;
        p.stage = "committing".into();
        Ok(())
    }
}
