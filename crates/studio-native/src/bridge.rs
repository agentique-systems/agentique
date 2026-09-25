//! Serialized, bounded worker boundary; reconstruction and durable IO never run in a frame.
use agq_modeling_agent::{AgentContext, ModelCommand};
use agq_modeling_repository::{CommitReceipt, Project};
use agq_modeling_view::{ElementInspector, ExplanationProjection, ViewProjection};
use agq_studio_platform::{CandidateProjection, ComparisonProjection, NativeConfig, ProjectHistory, SourceProjection, StudioPlatform};
use eframe::egui;
use std::{path::PathBuf, sync::mpsc::{self, Receiver, SyncSender}};

pub enum Output {
    Ready(Vec<Project>),
    History(ProjectHistory),
    Projection(ViewProjection),
    Inspector(ElementInspector),
    Explanation(ExplanationProjection),
    Source(SourceProjection),
    Comparison(ComparisonProjection),
    Candidate(CandidateProjection),
    Committed(CommitReceipt),
    Cancelled,
}
pub type Work = Box<dyn FnOnce(&mut StudioPlatform) -> agq_studio_platform::Result<Output> + Send>;
enum Request {
    Open(u64, NativeConfig, Option<PathBuf>),
    Work(u64, Work),
}
pub struct Reply { pub request: u64, pub result: Result<Output, String> }
pub struct Bridge {
    sender: SyncSender<Request>,
    pub replies: Receiver<Reply>,
    next: u64,
}
impl Bridge {
    pub fn new(ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Request>(16);
        let (reply, replies) = mpsc::channel();
        std::thread::Builder::new().name("agentique-modeling".into()).spawn(move || {
            let mut platform = None;
            while let Ok(request) = receiver.recv() {
                let (id, result) = match request {
                    Request::Open(id, config, bundle) => {
                        let opened = if let Some(bundle) = bundle { agq_studio_platform::install(&config, &bundle, |_| {}) } else { agq_studio_platform::open(&config, |_| {}) };
                        let result = opened.and_then(|value| { let projects = value.projects()?; platform = Some(value); Ok(Output::Ready(projects)) });
                        (id, result)
                    }
                    Request::Work(id, work) => (id, platform.as_mut().ok_or_else(|| agq_studio_platform::PlatformError::Invalid("Authenticated runtime is not open".into())).and_then(work)),
                };
                if reply.send(Reply { request: id, result: result.map_err(|e| e.to_string()) }).is_err() { break; }
                ctx.request_repaint();
            }
        }).expect("model worker thread");
        Self { sender, replies, next: 1 }
    }
    pub fn open(&mut self, config: NativeConfig, bundle: Option<PathBuf>) -> Result<u64,String> {
        let id = self.next; self.next += 1;
        self.sender.try_send(Request::Open(id, config, bundle)).map_err(|e| e.to_string())?;
        Ok(id)
    }
    pub fn work(&mut self, work: Work) -> Result<u64,String> {
        let id = self.next; self.next += 1;
        self.sender.try_send(Request::Work(id, work)).map_err(|e| e.to_string())?;
        Ok(id)
    }
}

/// All visual edit entry points construct this existing provider-neutral intent.
pub fn nested_part(context: AgentContext, owner: agq_kernel::ElementId, name: String, view: agq_modeling_view::ViewDefinition) -> Work {
    Box::new(move |platform| platform.propose(context, ModelCommand::CreatePartUsage { owner, name, definition: None }, &view).map(Output::Candidate))
}
