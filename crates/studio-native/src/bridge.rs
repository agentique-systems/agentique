//! Serialized, bounded worker boundary; reconstruction and durable IO never run in a frame.
use agq_modeling_agent::{AgentContext, ModelCommand};
use agq_modeling_repository::{CommitReceipt, Project};
use agq_modeling_view::{ElementInspector, ExplanationProjection, ViewProjection};
use agq_studio_platform::{
    BootstrapPhase, CandidateId, CandidateProjection, ComparisonProjection, NativeConfig,
    ProjectHistory, RevisionBinding, SourceProjection, StudioPlatform,
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
    History(ProjectHistory),
    HistoryRefresh(ProjectHistory),
    Projection(ViewProjection),
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
    Work(u64, WorkContext, bool, Work),
}
pub struct Reply {
    pub request: u64,
    pub context: Option<WorkContext>,
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
}
impl Bridge {
    pub fn new(ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Request>(16);
        let (reply, replies) = mpsc::channel();
        std::thread::Builder::new()
            .name("agentique-modeling".into())
            .spawn(move || {
                let mut platform = None;
                while let Ok(request) = receiver.recv() {
                    let (id, context, mutation, result) = match request {
                        Request::Open(id, config, bundle) => {
                            // A failed switch must never leave a previous repository silently active.
                            platform = None;
                            let progress = |phase| {
                                let _ = reply.send(Reply {
                                    request: id,
                                    context: None,
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
                            (id, None, false, result)
                        }
                        Request::Work(id, context, mutation, work) => {
                            let result = platform
                                .as_mut()
                                .ok_or_else(|| {
                                    agq_studio_platform::PlatformError::Invalid(
                                        "Authenticated runtime is not open".into(),
                                    )
                                })
                                .and_then(work);
                            (id, Some(context), mutation, result)
                        }
                    };
                    if reply
                        .send(Reply {
                            request: id,
                            context,
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
        }
    }
    pub fn open(&mut self, config: NativeConfig, bundle: Option<PathBuf>) -> Result<u64, String> {
        if self.mutation_pending() {
            return Err("Wait for the model operation before changing runtime".into());
        }
        let id = self.next;
        self.next += 1;
        self.sender
            .try_send(Request::Open(id, config, bundle))
            .map_err(|e| e.to_string())?;
        self.latest_open = Some(id);
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
            .try_send(Request::Work(id, context, mutation, work))
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
}

/// All visual edit entry points construct this existing provider-neutral intent.
pub fn nested_part(
    context: AgentContext,
    owner: agq_kernel::ElementId,
    name: String,
    view: agq_modeling_view::ViewDefinition,
) -> Work {
    propose(
        context,
        ModelCommand::CreatePartUsage {
            owner,
            name,
            definition: None,
        },
        view,
    )
}

pub fn propose(
    context: AgentContext,
    command: ModelCommand,
    view: agq_modeling_view::ViewDefinition,
) -> Work {
    Box::new(move |platform| {
        platform
            .propose(context, command, &view)
            .map(Output::Candidate)
    })
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
