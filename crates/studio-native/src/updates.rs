use crate::{app::{Candidate,ComparisonMode,StudioApp},bridge::Output,navigation::{Navigation,World}};
use agq_studio_platform::RevisionBinding;
use eframe::egui;
use std::time::Duration;

impl StudioApp {
    pub fn receive(&mut self) {
        while let Ok(reply)=self.bridge.replies.try_recv() {
            self.pending.remove(&reply.request);
            match reply.result {
                Err(error)=>{self.setup_reason=error.clone();self.status=error;}
                Ok(Output::Ready(projects))=>{
                    self.projects=projects;
                    if self.fixture.is_none() && let Some(project)=self.restore.as_ref().and_then(|s|s.project).filter(|id|self.projects.iter().any(|p|p.id==*id)) {self.open_project(project);}
                    else {self.setup_reason="Runtime authenticated. Choose a project.".into();}
                }
                Ok(Output::History(history)) if reply.request==self.project_request=>{
                    let branch=history.branches.iter().find(|b|b.id==history.project.default_branch).cloned();
                    if let Some(branch)=branch {
                        let revision=self.restore.as_ref().filter(|s|s.project==Some(history.project.id)).map(|s|s.revision).filter(|id|history.revisions.iter().any(|r|r.revision_id==*id)).unwrap_or(branch.head);
                        self.binding=Some(RevisionBinding {project:history.project.id,revision});self.branch=Some(branch.id);self.history=Some(history);
                        self.fixture=None;self.ready=true;self.focus=None;self.world=World::System;self.navigation=Navigation::default();
                        self.candidate=None;self.compare_before=None;self.comparison=ComparisonMode::Current;self.layout=Default::default();self.selection.clear();
                        self.request_projection();
                    }
                }
                Ok(Output::Projection(projection)) if reply.request==self.scene_request=>{
                    self.projection=projection;self.rebuild();self.fit_pending=true;
                    if let Some(restore)=self.restore.take() && restore.project==self.project_id() && restore.revision==self.projection.revision_id {
                        self.world=restore.world;self.focus=restore.focus.filter(|id|self.projection.nodes.iter().any(|n|n.id==*id));self.camera=restore.camera;self.layout=restore.layout;self.rebuild();self.fit_pending=false;
                    }
                    self.record_location();self.status="Revision restored in process".into();
                }
                Ok(Output::Inspector(inspector)) if reply.request==self.inspector_request=>self.inspector=Some(inspector),
                Ok(Output::Explanation(explanation)) if reply.request==self.explanation_request=>self.explanation=Some(explanation),
                Ok(Output::Source(source)) if reply.request==self.source_request=>self.source=Some(source),
                Ok(Output::Comparison(comparison)) if reply.request==self.scene_request=>{
                    self.projection=comparison.after;self.compare_before=Some(comparison.before);self.comparison=ComparisonMode::Diff;self.rebuild();self.fit_pending=true;
                }
                Ok(Output::Candidate(candidate))=>{
                    self.candidate=Some(Candidate {id:Some(candidate.id),phase:Some(candidate.phase),before:self.projection.clone(),after:candidate.projection,source:format!("{}\n\n{}",candidate.source_preview.path,candidate.source_preview.after)});
                    self.comparison=ComparisonMode::Diff;self.rebuild();self.fit_pending=true;self.status="Candidate revision ready for review".into();
                }
                Ok(Output::Committed(receipt))=>{
                    if let Some(binding)=self.binding {self.binding=Some(RevisionBinding {revision:receipt.revision_id,..binding});}
                    self.candidate=None;self.compare_before=None;self.comparison=ComparisonMode::Current;self.request_projection();
                    self.status="Candidate durably committed".into();
                }
                Ok(Output::Cancelled)=>{self.candidate=None;self.comparison=ComparisonMode::Current;self.rebuild();self.status="Uncommitted candidate cancelled".into();}
                Ok(_)=>{} // Superseded read requests cannot populate another view/selection.
            }
        }
    }
    pub fn open_project(&mut self,id:agq_modeling_repository::ProjectId) {
        self.project_request=self.enqueue(Box::new(move|platform|platform.history(id).map(Output::History)));
    }
    pub fn capture(&mut self,ctx:&egui::Context) {
        if self.args.screenshot.is_some() && !self.capture_requested && self.frame_number>=20 {
            self.capture_requested=true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshot=ctx.input(|i|i.events.iter().find_map(|event|if let egui::Event::Screenshot{image,..}=event {Some(image.clone())}else{None}));
        if let Some(image)=screenshot && let Some(path)=&self.args.screenshot {
            if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}
            let bytes:Vec<u8>=image.pixels.iter().flat_map(|p|p.to_array()).collect();
            match image::save_buffer(path,&bytes,image.size[0] as u32,image.size[1] as u32,image::ColorType::Rgba8) {
                Ok(())=>{self.capture_done=true;println!("Native surface captured: {}",path.display());}
                Err(error)=>{self.status=format!("Screenshot failed: {error}");eprintln!("{}",self.status);self.capture_done=true;}
            }
        }
        if self.args.screenshot.is_some()&&!self.capture_done {ctx.request_repaint_after(Duration::from_millis(16));}
        let finished=self.args.frames.is_some_and(|frames|self.frame_number>=frames) || (self.args.frames.is_none()&&self.args.screenshot.is_some()&&self.capture_done);
        if finished && (self.args.screenshot.is_none()||self.capture_done) {
            let stats=self.gpu_stats.lock().map(|s|s.clone()).unwrap_or_default();
            let report=serde_json::json!({"fixture":self.fixture,"adapter":self.adapter,"frame_count":self.frame_number,"frame_interval_median_ms":self.timing.median_ms(),"frame_interval_p95_ms":self.timing.p95_ms(),"scene_build_ms":self.timing.scene_ms,"layout_included_in_scene_build":true,"hit_test_us":self.timing.hit_us,"input_to_ui_frame_ms":self.timing.input_to_frame_ms,"gpu_upload_cpu_ms":stats.upload_ms,"gpu_uploaded_bytes":stats.uploaded_bytes,"gpu_upload_count":stats.uploads,"gpu_instances":stats.instances,"scene_draw_calls":stats.draw_calls,"visible_nodes":self.timing.visible_nodes,"total_nodes":self.scene.nodes.len(),"total_edges":self.scene.edges.len(),"gpu_timestamp_ms":null,"note":"Actual native wgpu frames, vsync enabled; frame interval is not GPU timestamp or measured physical input latency."});
            if let Some(path)=&self.args.metrics {if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}let _=std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap());}
            println!("{report}");ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
