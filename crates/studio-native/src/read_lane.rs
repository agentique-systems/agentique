//! Bounded reads of a retained immutable revision, independent of reconstruction.
//! A mutex protects only the tiny mailbox; semantic queries run without that lock.
use crate::bridge::{Output, Reply};
use agq_kernel::ElementId;
use agq_studio_platform::{RevisionBinding, StudioRevisionReader};
use eframe::egui;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, mpsc},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PanelRead {
    Projection,
    Inspector,
    Explain,
    Source,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadScope {
    pub epoch: u64,
    pub binding: RevisionBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadContext {
    pub scope: ReadScope,
    pub panel: PanelRead,
    pub element: ElementId,
}

#[derive(Clone)]
struct Request {
    id: u64,
    context: ReadContext,
    definition: Option<agq_modeling_view::ViewDefinition>,
}

#[derive(Default)]
struct Mailbox {
    scope: Option<ReadScope>,
    reader: Option<Arc<StudioRevisionReader>>,
    // At most one waiting projection and three panel reads, plus one executing read.
    pending: BTreeMap<PanelRead, Request>,
}
impl Mailbox {
    fn take(&mut self) -> Option<(Request, Arc<StudioRevisionReader>)> {
        let reader = self.reader.as_ref()?.clone();
        let panel = self
            .pending
            .values()
            .min_by_key(|request| request.id)?
            .context
            .panel;
        Some((self.pending.remove(&panel)?, reader))
    }
}

pub struct ReadLane {
    mailbox: Arc<Mutex<Mailbox>>,
    wake: mpsc::SyncSender<()>,
    replies: mpsc::Sender<Reply>,
    ctx: egui::Context,
}

impl ReadLane {
    pub fn new(ctx: egui::Context, replies: mpsc::Sender<Reply>) -> Self {
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let worker = mailbox.clone();
        let sender = replies.clone();
        let repaint = ctx.clone();
        let (wake, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("agentique-revision-reader".into())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    loop {
                        let next = worker.lock().expect("read mailbox").take();
                        let Some((request, reader)) = next else { break };
                        let result = execute_read(|| {
                            if reader.binding() != request.context.scope.binding {
                                return Err(agq_studio_platform::PlatformError::Invalid(
                                    "Pinned reader does not match the requested revision".into(),
                                ));
                            }
                            match request.context.panel {
                                PanelRead::Projection => reader
                                    .project(request.definition.as_ref().ok_or_else(|| {
                                        agq_studio_platform::PlatformError::Invalid(
                                            "Projection read is missing its exact view definition"
                                                .into(),
                                        )
                                    })?)
                                    .map(Output::Projection),
                                PanelRead::Inspector => reader
                                    .inspect(request.context.element)
                                    .map(Output::Inspector),
                                PanelRead::Explain => reader
                                    .explain(request.context.element)
                                    .map(Output::Explanation),
                                PanelRead::Source => {
                                    reader.source(request.context.element).map(Output::Source)
                                }
                            }
                        });
                        if sender.send(completion(request, result)).is_err() {
                            return;
                        }
                        repaint.request_repaint();
                    }
                }
            })
            .expect("immutable revision reader thread");
        Self {
            mailbox,
            wake,
            replies,
            ctx,
        }
    }

    /// Switching scopes drops queued reads and the old capability. An executing
    /// read retains its own immutable handle until its fenced terminal response.
    pub fn reset(&self, scope: Option<ReadScope>) {
        let cancelled = {
            let mut mailbox = self.mailbox.lock().expect("read mailbox");
            mailbox.scope = scope;
            mailbox.reader = None;
            std::mem::take(&mut mailbox.pending)
        };
        for request in cancelled.into_values() {
            self.finish(
                request,
                "Read superseded by a different revision or runtime",
            );
        }
    }

    pub fn scope(&self) -> Option<ReadScope> {
        self.mailbox.lock().expect("read mailbox").scope
    }

    pub fn ready(&self, scope: ReadScope) -> bool {
        let mailbox = self.mailbox.lock().expect("read mailbox");
        mailbox.scope == Some(scope) && mailbox.reader.is_some()
    }

    pub fn install(&self, scope: ReadScope, reader: StudioRevisionReader) -> Result<(), String> {
        {
            let mut mailbox = self.mailbox.lock().expect("read mailbox");
            if mailbox.scope != Some(scope) || reader.binding() != scope.binding {
                return Err("Discarded a reader from another revision or runtime".into());
            }
            mailbox.reader = Some(Arc::new(reader));
        }
        self.wake()
    }

    pub fn fail(&self, scope: ReadScope, error: &str) {
        let cancelled = {
            let mut mailbox = self.mailbox.lock().expect("read mailbox");
            if mailbox.scope != Some(scope) {
                return;
            }
            mailbox.reader = None;
            std::mem::take(&mut mailbox.pending)
        };
        for request in cancelled.into_values() {
            self.finish(request, error);
        }
    }

    pub fn cancel_pending(&self) {
        // A selection change invalidates panels, never an independent view read.
        let cancelled = {
            let mut mailbox = self.mailbox.lock().expect("read mailbox");
            let projection = mailbox.pending.remove(&PanelRead::Projection);
            let cancelled = std::mem::take(&mut mailbox.pending);
            if let Some(projection) = projection {
                mailbox.pending.insert(PanelRead::Projection, projection);
            }
            cancelled
        };
        for request in cancelled.into_values() {
            self.finish(
                request,
                "Read superseded by a new selection or dismissed panel",
            );
        }
    }

    pub fn request(&self, id: u64, context: ReadContext) -> Result<(), String> {
        self.enqueue(Request {
            id,
            context,
            definition: None,
        })
    }

    pub fn project(
        &self,
        id: u64,
        scope: ReadScope,
        definition: agq_modeling_view::ViewDefinition,
    ) -> Result<(), String> {
        self.enqueue(Request {
            id,
            // Projection requests have no selected element. This field is never
            // interpreted for Projection; exact scope and definition bind them.
            context: ReadContext {
                scope,
                panel: PanelRead::Projection,
                element: ElementId::from_u128(0),
            },
            definition: Some(definition),
        })
    }

    fn enqueue(&self, request: Request) -> Result<(), String> {
        let replaced = {
            let mut mailbox = self.mailbox.lock().expect("read mailbox");
            if mailbox.scope != Some(request.context.scope) {
                return Err("Read-only access has not been pinned for this revision".into());
            }
            mailbox.pending.insert(request.context.panel, request)
        };
        if let Some(request) = replaced {
            self.finish(request, "Read superseded by newer panel input");
        }
        self.wake()
    }

    fn wake(&self) -> Result<(), String> {
        match self.wake.try_send(()) {
            Ok(()) | Err(mpsc::TrySendError::Full(())) => Ok(()),
            Err(mpsc::TrySendError::Disconnected(())) => {
                if let Some(scope) = self.scope() {
                    self.fail(scope, "Immutable read worker is unavailable");
                }
                Err("Immutable read worker is unavailable".into())
            }
        }
    }

    fn finish(&self, request: Request, reason: &str) {
        let _ = self.replies.send(completion(request, Err(reason.into())));
        self.ctx.request_repaint();
    }
}

fn completion(request: Request, result: Result<Output, String>) -> Reply {
    Reply {
        request: request.id,
        epoch: request.context.scope.epoch,
        context: None,
        read: Some(request.context),
        mutation: false,
        terminal: true,
        result,
    }
}

fn execute_read(
    work: impl FnOnce() -> agq_studio_platform::Result<Output>,
) -> Result<Output, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
        .map_err(|_| {
            "Immutable read failed unexpectedly; current revision is unchanged".to_string()
        })?
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_modeling_repository::{ProjectId, ProjectRevisionId};

    fn scope() -> ReadScope {
        ReadScope {
            epoch: 17,
            binding: RevisionBinding {
                project: ProjectId::new(),
                revision: ProjectRevisionId::new(),
            },
        }
    }
    fn context(scope: ReadScope, panel: PanelRead, element: u128) -> ReadContext {
        ReadContext {
            scope,
            panel,
            element: ElementId::from_u128(element),
        }
    }

    #[test]
    fn projection_slot_is_bounded_and_selection_cancellation_preserves_view_work() {
        let (send, replies) = mpsc::channel();
        let lane = ReadLane::new(egui::Context::default(), send);
        let scope = scope();
        lane.reset(Some(scope));
        for id in 1..=1000 {
            let mut definition = agq_modeling_view::ViewDefinition::architecture();
            definition.focus = Some(ElementId::from_u128(id as u128));
            lane.project(id, scope, definition).unwrap();
        }
        lane.request(1001, context(scope, PanelRead::Inspector, 1))
            .unwrap();
        lane.cancel_pending();
        let mailbox = lane.mailbox.lock().unwrap();
        assert_eq!(mailbox.pending.len(), 1);
        let request = &mailbox.pending[&PanelRead::Projection];
        assert_eq!(request.id, 1000);
        assert_eq!(
            request.definition.as_ref().unwrap().focus,
            Some(ElementId::from_u128(1000))
        );
        assert_eq!(replies.try_iter().count(), 1000);
    }

    #[test]
    fn latest_panel_slots_bound_pending_work_and_terminate_every_replaced_id() {
        let (send, replies) = mpsc::channel();
        let lane = ReadLane::new(egui::Context::default(), send);
        let scope = scope();
        lane.reset(Some(scope)); // No authenticated reader: queued requests cannot run.
        for id in 1..=100 {
            lane.request(id, context(scope, PanelRead::Inspector, id as u128))
                .unwrap();
        }
        lane.request(101, context(scope, PanelRead::Explain, 100))
            .unwrap();
        lane.request(102, context(scope, PanelRead::Source, 100))
            .unwrap();
        assert_eq!(lane.mailbox.lock().unwrap().pending.len(), 3);
        assert!(!lane.ready(scope));
        let cancelled: Vec<_> = replies.try_iter().collect();
        assert_eq!(
            cancelled
                .iter()
                .map(|reply| reply.request)
                .collect::<Vec<_>>(),
            (1..100).collect::<Vec<_>>()
        );
        assert!(
            cancelled
                .iter()
                .all(|reply| reply.terminal && !reply.mutation && reply.result.is_err())
        );
        lane.fail(scope, "Authority refused reader pin");
        let mut remaining: Vec<_> = replies.try_iter().map(|reply| reply.request).collect();
        remaining.sort();
        assert_eq!(remaining, vec![100, 101, 102]);
        assert!(lane.mailbox.lock().unwrap().pending.is_empty());
    }

    #[test]
    fn runtime_or_revision_reset_cancels_waiting_reads_and_refuses_old_scope() {
        let (send, replies) = mpsc::channel();
        let lane = ReadLane::new(egui::Context::default(), send);
        let old = scope();
        lane.reset(Some(old));
        lane.request(1, context(old, PanelRead::Inspector, 1))
            .unwrap();
        let new = ReadScope {
            epoch: old.epoch + 1,
            ..old
        };
        lane.reset(Some(new));
        let reply = replies.try_recv().unwrap();
        assert_eq!(reply.request, 1);
        assert_eq!(reply.epoch, old.epoch);
        assert!(reply.terminal);
        assert!(lane.request(2, context(old, PanelRead::Source, 1)).is_err());
        let different_revision = ReadScope {
            binding: RevisionBinding {
                revision: ProjectRevisionId::new(),
                ..new.binding
            },
            ..new
        };
        assert!(
            lane.request(3, context(different_revision, PanelRead::Inspector, 1))
                .is_err()
        );
        lane.request(4, context(new, PanelRead::Inspector, 1))
            .unwrap();
        lane.reset(None);
        assert_eq!(replies.try_recv().unwrap().request, 4);
    }

    #[test]
    fn failed_or_panicking_read_produces_a_terminal_failure_not_a_pending_leak() {
        for panic in [false, true] {
            let result = execute_read(|| {
                assert!(!panic, "test read failure");
                Err(agq_studio_platform::PlatformError::Invalid(
                    "missing element".into(),
                ))
            });
            let reply = completion(
                Request {
                    id: 9,
                    context: context(scope(), PanelRead::Inspector, 1),
                    definition: None,
                },
                result,
            );
            assert!(reply.terminal && !reply.mutation && reply.result.is_err());
        }
    }
}
