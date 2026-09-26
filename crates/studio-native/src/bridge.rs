//! Serialized mutations and an independent immutable read lane. No semantic work runs in a frame.
use crate::read_lane::{PanelRead, ReadContext, ReadLane, ReadScope};
use agq_modeling_agent::{AgentContext, ModelCommand};
use agq_modeling_repository::{CommitReceipt, Project};
use agq_modeling_view::{ElementInspector, ExplanationProjection, ViewProjection};
use agq_studio_platform::{
    BootstrapPhase, CandidateId, CandidateProjection, ComparisonProjection, NativeConfig,
    ProjectHistory, RevisionBinding, SourceProjection, StudioPlatform, StudioRevisionReader,
};
use eframe::egui;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
};

pub enum Output {
    Progress(BootstrapPhase),
    Ready(Vec<Project>),
    ProjectCreated {
        project: Project,
        projects: Vec<Project>,
        database: PathBuf,
    },
    History(ProjectHistory),
    HistoryRefresh(ProjectHistory),
    Projection(ViewProjection),
    Reader(StudioRevisionReader),
    Inspector(ElementInspector),
    Explanation(ExplanationProjection),
    Source(SourceProjection),
    Comparison(ComparisonProjection),
    Candidate(CandidateProjection),
    CandidateView(CandidateProjection, ViewProjection),
    /// Lifecycle recovery never replaces a disposable scene or selection.
    CandidateLifecycle(CandidateProjection),
    Committed(CommitReceipt),
    Cancelled,
    /// Cooperative source interruption; no candidate was retained or published.
    PreparationCancelled,
}

/// A worker response can update only the project/candidate context that requested it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkContext {
    pub binding: Option<RevisionBinding>,
    pub candidate: Option<CandidateId>,
    pub fixture: Option<String>,
}
impl WorkContext {
    pub fn matches(&self, current: &Self) -> bool {
        self == current
    }
}

pub type Work = Box<dyn FnOnce(&mut StudioPlatform) -> agq_studio_platform::Result<Output> + Send>;
enum Request {
    Open(u64, NativeConfig, Option<PathBuf>),
    Work(u64, u64, WorkContext, bool, Work),
}
pub struct Reply {
    pub request: u64,
    pub epoch: u64,
    pub context: Option<WorkContext>,
    pub read: Option<ReadContext>,
    pub mutation: bool,
    pub terminal: bool,
    pub result: Result<Output, String>,
}
pub struct Bridge {
    sender: SyncSender<Request>,
    pub replies: Receiver<Reply>,
    next: u64,
    mutations: BTreeSet<u64>,
    latest_open: Option<u64>,
    reads: ReadLane,
    reader_pin: Option<(u64, ReadScope)>,
}
impl Bridge {
    pub fn new(ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Request>(16);
        let (reply, replies) = mpsc::channel();
        let reads = ReadLane::new(ctx.clone(), reply.clone());
        std::thread::Builder::new()
            .name("agentique-modeling".into())
            .spawn(move || {
                let mut platform = None;
                let mut open_epoch = 0;
                while let Ok(request) = receiver.recv() {
                    let (id, epoch, context, mutation, result) = match request {
                        Request::Open(id, config, bundle) => {
                            // A failed switch must never leave a previous repository silently active.
                            platform = None;
                            open_epoch = id;
                            let progress = |phase| {
                                let _ = reply.send(Reply {
                                    request: id,
                                    epoch: id,
                                    context: None,
                                    read: None,
                                    mutation: false,
                                    terminal: false,
                                    result: Ok(Output::Progress(phase)),
                                });
                                ctx.request_repaint();
                            };
                            let opened = if let Some(bundle) = bundle {
                                agq_studio_platform::install(&config, &bundle, progress)
                            } else {
                                agq_studio_platform::open(&config, progress)
                            };
                            let result = opened.and_then(|value| {
                                let projects = value.projects()?;
                                platform = Some(value);
                                Ok(Output::Ready(projects))
                            });
                            (id, id, None, false, result)
                        }
                        Request::Work(id, epoch, context, mutation, work) => {
                            let result = if epoch != open_epoch {
                                Err(agq_studio_platform::PlatformError::Invalid(
                                    "Work belongs to an earlier runtime opening".into(),
                                ))
                            } else {
                                platform
                                    .as_mut()
                                    .ok_or_else(|| {
                                        agq_studio_platform::PlatformError::Invalid(
                                            "Authenticated runtime is not open".into(),
                                        )
                                    })
                                    .and_then(work)
                            };
                            (id, epoch, Some(context), mutation, result)
                        }
                    };
                    if reply
                        .send(Reply {
                            request: id,
                            epoch,
                            context,
                            read: None,
                            mutation,
                            terminal: true,
                            result: result.map_err(|e| e.to_string()),
                        })
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("model worker thread");
        Self {
            sender,
            replies,
            next: 1,
            mutations: BTreeSet::new(),
            latest_open: None,
            reads,
            reader_pin: None,
        }
    }
    pub fn open(&mut self, config: NativeConfig, bundle: Option<PathBuf>) -> Result<u64, String> {
        if self.mutation_pending() {
            return Err("Wait for the model operation before changing runtime".into());
        }
        let id = self.next;
        self.next += 1;
        // An attempted runtime change revokes native presentation access before
        // queueing, even if enqueueing or later authentication fails.
        self.latest_open = Some(id);
        self.clear_reader();
        self.sender
            .try_send(Request::Open(id, config, bundle))
            .map_err(|e| e.to_string())?;
        Ok(id)
    }
    pub fn work(
        &mut self,
        work: Work,
        context: WorkContext,
        mutation: bool,
    ) -> Result<u64, String> {
        if mutation && self.mutation_pending() {
            return Err("A model operation is already running".into());
        }
        let id = self.next;
        self.next += 1;
        self.sender
            .try_send(Request::Work(id, self.epoch(), context, mutation, work))
            .map_err(|e| e.to_string())?;
        if mutation {
            self.mutations.insert(id);
        }
        Ok(id)
    }
    pub fn mutation_pending(&self) -> bool {
        !self.mutations.is_empty()
    }
    pub fn complete(&mut self, request: u64) {
        self.mutations.remove(&request);
    }
    pub fn current_open(&self, request: u64) -> bool {
        self.latest_open == Some(request)
    }
    pub fn epoch(&self) -> u64 {
        self.latest_open.unwrap_or(0)
    }
    pub fn current_epoch(&self, epoch: u64) -> bool {
        epoch == self.epoch()
    }

    /// Queue before enabling interaction with a newly accepted real projection.
    /// The serial worker alone resolves and grants the opaque read capability.
    pub fn pin_reader(
        &mut self,
        binding: RevisionBinding,
        context: WorkContext,
    ) -> Result<Option<u64>, String> {
        if context.fixture.is_some() || context.binding != Some(binding) {
            return Err("Only the displayed real revision can request a reader".into());
        }
        let scope = ReadScope {
            epoch: self.epoch(),
            binding,
        };
        if self.reads.ready(scope) || self.reader_pin.is_some_and(|(_, pending)| pending == scope) {
            return Ok(None);
        }
        self.reader_pin = None;
        self.reads.reset(Some(scope));
        let request = match self.work(
            Box::new(move |platform| platform.revision_reader(binding).map(Output::Reader)),
            context,
            false,
        ) {
            Ok(request) => request,
            Err(error) => {
                // No pin will arrive: a panel must fail immediately rather than
                // wait forever in an unminted read scope.
                self.reads.reset(None);
                return Err(error);
            }
        };
        self.reader_pin = Some((request, scope));
        Ok(Some(request))
    }

    pub fn reader_pin(&self, request: u64) -> Option<ReadScope> {
        self.reader_pin
            .filter(|(id, _)| *id == request)
            .map(|(_, scope)| scope)
    }

    pub fn install_reader(
        &mut self,
        request: u64,
        reader: StudioRevisionReader,
    ) -> Result<(), String> {
        let scope = self
            .reader_pin(request)
            .ok_or("Reader pin was superseded")?;
        self.reader_pin = None;
        if !self.current_epoch(scope.epoch) {
            self.reads
                .fail(scope, "Reader pin belongs to an earlier runtime opening");
            return Err("Reader pin belongs to an earlier runtime opening".into());
        }
        let result = self.reads.install(scope, reader);
        if let Err(error) = &result {
            self.reads.fail(scope, error);
        }
        result
    }

    pub fn fail_reader(&mut self, request: u64, error: &str) {
        if let Some(scope) = self.reader_pin(request) {
            self.reader_pin = None;
            self.reads.fail(scope, error);
        }
    }

    pub fn clear_reader(&mut self) {
        self.reader_pin = None;
        self.reads.reset(None);
    }

    pub fn cancel_reads(&self) {
        self.reads.cancel_pending();
    }

    pub fn read(
        &mut self,
        binding: RevisionBinding,
        element: agq_kernel::ElementId,
        panel: PanelRead,
    ) -> Result<u64, String> {
        let request = self.next;
        self.next += 1;
        self.reads.request(
            request,
            ReadContext {
                scope: ReadScope {
                    epoch: self.epoch(),
                    binding,
                },
                panel,
                element,
            },
        )?;
        Ok(request)
    }

    pub fn project_read(
        &mut self,
        binding: RevisionBinding,
        definition: agq_modeling_view::ViewDefinition,
    ) -> Result<u64, String> {
        let request = self.next;
        self.next += 1;
        self.reads.project(
            request,
            ReadScope {
                epoch: self.epoch(),
                binding,
            },
            definition,
        )?;
        Ok(request)
    }
}

/// All visual edit entry points construct this existing provider-neutral intent.
pub fn nested_part(
    context: AgentContext,
    owner: agq_kernel::ElementId,
    name: String,
    view: agq_modeling_view::ViewDefinition,
    control: agq_studio_platform::CompilationControl,
) -> Work {
    propose(
        context,
        ModelCommand::CreatePartUsage {
            owner,
            name,
            definition: None,
        },
        view,
        control,
    )
}

pub fn propose(
    context: AgentContext,
    command: ModelCommand,
    view: agq_modeling_view::ViewDefinition,
    control: agq_studio_platform::CompilationControl,
) -> Work {
    Box::new(
        move |platform| match platform.propose_controlled(context, command, &view, &control) {
            Err(error) if error.is_cancelled() => Ok(Output::PreparationCancelled),
            result => result.map(Output::Candidate),
        },
    )
}

/// Compare a candidate and its base through the same lens; world changes cannot
/// manufacture apparent removals by comparing two different selection scopes.
pub fn candidate_view(
    platform: &mut StudioPlatform,
    id: CandidateId,
    view: &agq_modeling_view::ViewDefinition,
) -> agq_studio_platform::Result<Output> {
    let candidate = platform.candidate(id, view)?;
    let before = platform.project(candidate.base, view)?;
    Ok(Output::CandidateView(candidate, before))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_modeling_repository::{ProjectId, ProjectRevisionId};

    #[test]
    fn refused_pin_enqueue_cannot_leave_panel_requests_waiting_without_a_capability() {
        let mut bridge = Bridge::new(egui::Context::default());
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        bridge.sender = sender;
        let binding = RevisionBinding {
            project: ProjectId::new(),
            revision: ProjectRevisionId::new(),
        };
        let context = WorkContext {
            binding: Some(binding),
            candidate: None,
            fixture: None,
        };
        assert!(bridge.pin_reader(binding, context).is_err());
        assert!(bridge.reader_pin.is_none());
        assert!(bridge.reads.scope().is_none());
        assert!(
            bridge
                .read(
                    binding,
                    agq_kernel::ElementId::from_u128(1),
                    PanelRead::Inspector
                )
                .is_err()
        );
    }

    #[test]
    fn even_failed_runtime_enqueue_invalidates_waiting_reads_and_old_epoch() {
        let mut bridge = Bridge::new(egui::Context::default());
        let binding = RevisionBinding {
            project: ProjectId::new(),
            revision: ProjectRevisionId::new(),
        };
        let context = WorkContext {
            binding: Some(binding),
            candidate: None,
            fixture: None,
        };
        let pin = bridge.pin_reader(binding, context).unwrap().unwrap();
        let read = bridge
            .read(
                binding,
                agq_kernel::ElementId::from_u128(1),
                PanelRead::Inspector,
            )
            .unwrap();
        let epoch = bridge.epoch();
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        bridge.sender = sender;
        let config =
            NativeConfig::for_root(std::env::temp_dir(), Some(std::env::temp_dir())).unwrap();
        assert!(bridge.open(config, None).is_err());
        assert!(!bridge.current_epoch(epoch));
        assert!(bridge.reader_pin(pin).is_none());
        assert!(bridge.reads.scope().is_none());
        let reply = bridge
            .replies
            .try_iter()
            .find(|reply| reply.request == read)
            .unwrap();
        assert!(reply.terminal && reply.result.is_err());
        assert_eq!(reply.epoch, epoch);
    }

    #[test]
    fn replies_cannot_cross_project_revision_fixture_or_candidate_context() {
        let scope = WorkContext {
            binding: Some(RevisionBinding {
                project: ProjectId::new(),
                revision: ProjectRevisionId::new(),
            }),
            candidate: None,
            fixture: None,
        };
        assert!(scope.matches(&scope));
        let mut changed = scope.clone();
        changed.binding.as_mut().unwrap().project = ProjectId::new();
        assert!(!scope.matches(&changed));
        changed = scope.clone();
        changed.binding.as_mut().unwrap().revision = ProjectRevisionId::new();
        assert!(!scope.matches(&changed));
        changed = scope.clone();
        changed.fixture = Some("architecture".into());
        assert!(!scope.matches(&changed));
        changed = scope.clone();
        changed.candidate =
            Some(serde_json::from_str("\"00000000-0000-0000-0000-000000000001\"").unwrap());
        assert!(!scope.matches(&changed));
    }

    #[test]
    fn unresolved_worker_request_blocks_runtime_switch_and_keeps_its_context() {
        let mut bridge = Bridge::new(egui::Context::default());
        let scope = WorkContext {
            binding: None,
            candidate: None,
            fixture: None,
        };
        let id = bridge
            .work(
                Box::new(|_| panic!("work ran without authenticated service")),
                scope.clone(),
                true,
            )
            .unwrap();
        assert!(bridge.mutation_pending());
        let config = NativeConfig::for_root(
            std::env::temp_dir(),
            Some(std::env::temp_dir().join("agq-missing-test-runtime")),
        )
        .unwrap();
        assert!(bridge.open(config, None).is_err());
        let reply = bridge
            .replies
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(reply.request, id);
        assert_eq!(reply.context, Some(scope));
        assert!(reply.terminal);
        assert!(reply.mutation);
        assert!(reply.result.is_err());
        bridge.complete(reply.request);
        assert!(!bridge.mutation_pending());
    }

    #[test]
    fn failed_bootstrap_emits_nonterminal_progress_before_terminal_error() {
        let mut bridge = Bridge::new(egui::Context::default());
        let missing = std::env::temp_dir().join(format!(
            "absent-native-runtime-{}",
            ProjectRevisionId::new()
        ));
        let mut config = NativeConfig::for_root(missing.clone(), Some(missing.clone())).unwrap();
        config.runtime.bundle = Some(missing.join("runtime.agq-runtime"));
        let id = bridge.open(config, None).unwrap();
        let mut progress = false;
        loop {
            let reply = bridge
                .replies
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(reply.request, id);
            assert!(bridge.current_open(id));
            if reply.terminal {
                assert!(reply.result.is_err());
                break;
            }
            assert!(matches!(reply.result, Ok(Output::Progress(_))));
            progress = true;
        }
        assert!(progress);
    }
}
