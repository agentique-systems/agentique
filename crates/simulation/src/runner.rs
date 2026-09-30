//! A run on a background thread, so the Studio never waits for it
//! (ROADMAP §4.14). The UI thread polls [`BackgroundRun::try_result`] each
//! frame; [`BackgroundRun::cancel`] ends the run at the next event.

use crate::compile::Program;
use crate::engine::Answers;
use crate::result::RunResult;
use crate::{Request, run};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// A run in progress.
pub struct BackgroundRun {
    receiver: Receiver<RunResult>,
    cancel: Arc<AtomicBool>,
    done: bool,
}

impl BackgroundRun {
    /// Starts the run. The program is the run's own snapshot.
    pub fn start(
        program: Program,
        model_digest: String,
        request: Request,
        answers: Answers,
    ) -> BackgroundRun {
        let (sender, receiver) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::Builder::new()
            .name("agentique-run".into())
            .spawn(move || {
                let result = run(&program, model_digest, &request, answers, flag);
                let _ = sender.send(result);
            })
            .expect("a run thread starts");
        BackgroundRun {
            receiver,
            cancel,
            done: false,
        }
    }

    /// Asks the run to stop; it ends with the status `cancelled` at its next
    /// event (a live call in flight is abandoned).
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The result, once the run has ended. Never blocks.
    pub fn try_result(&mut self) -> Option<RunResult> {
        if self.done {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(result) => {
                self.done = true;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.done = true;
                None
            }
        }
    }

    pub fn running(&self) -> bool {
        !self.done
    }
}

impl Drop for BackgroundRun {
    fn drop(&mut self) {
        self.cancel();
    }
}
