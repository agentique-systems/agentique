use crate::{app::{Candidate,ComparisonMode,StudioApp,fixture_projection},bridge::{Output,Work},commands::{self,CommandContext,CommandId},navigation::{Location,World}};
use agq_kernel::ElementId;
use agq_modeling_view::{FeatureCounts,ViewDefinition,ViewEdge,ViewNode,ViewOrigin,RelationshipFamily};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{Point,SceneTarget,fixtures};
use eframe::egui::{self,Key,Modifiers};
use std::collections::BTreeSet;

impl StudioApp {
    pub fn context(&self)->CommandContext {
        CommandContext {selected:self.selection.primary.is_some(),candidate:self.candidate.is_some(),validated:self.candidate.as_ref().is_some_and(|c|matches!(c.phase,Some(agq_studio_platform::CandidatePhase::Validated|agq_studio_platform::CandidatePhase::CommitUnresolved))),live:self.binding.is_some()&&self.fixture.is_none(),busy:!self.pending.is_empty()}
    }
    pub fn enqueue(&mut self,work:Work)->u64 {
        match self.bridge.work(work) {Ok(id)=>{self.pending.insert(id);id},Err(e)=>{self.status=e;0}}
    }
    pub fn definition(&self)->ViewDefinition {
        let mut view=match self.world {World::System=>ViewDefinition::architecture(),World::Requirements=>ViewDefinition::requirements(),_=>ViewDefinition::semantic_graph()};
        view.focus=self.focus; view.include_standard_library=self.include_standard;
        view.relationship_families=self.families.iter().copied().collect();
        view
    }
    pub fn selected_element(&self)->Option<ElementId> {self.selection.element(&self.scene)}
    pub fn select(&mut self,target:SceneTarget,extend:bool) {
        self.selection.select(target,extend); self.batch_key=None;
        self.inspector=None; self.explanation=None;self.source=None;
        self.inspector_request=0;self.explanation_request=0;self.source_request=0;
        if let (Some(binding),Some(element))=(self.binding,self.selected_element()) {
            let candidate=self.visible_candidate_id();
            // Ghosts preserve the revision they came from, including semantic inspection.
            let revision=self.selection.primary.as_ref().and_then(|target|match target {
                SceneTarget::Node(id)|SceneTarget::Container(id)=>self.scene.node(*id).map(|n|n.semantic.revision_id),
                SceneTarget::Port(id)=>self.scene.ports.iter().find(|p|p.id==*id).map(|p|p.revision_id),
                SceneTarget::Edge(id)=>self.scene.edges.iter().find(|e|e.semantic.id==*id).map(|e|e.semantic.revision_id),
            }).unwrap_or(binding.revision);
            let binding=agq_studio_platform::RevisionBinding {revision,..binding};
            let candidate=candidate.filter(|_|self.candidate.as_ref().is_some_and(|c|c.after.revision_id==revision));
            self.inspector_request=self.enqueue(Box::new(move|platform|if let Some(id)=candidate {platform.inspect_candidate(id,element).map(Output::Inspector)}else{platform.inspect(binding,element).map(Output::Inspector)}));
        }
    }
    pub fn visible_candidate_id(&self)->Option<agq_studio_platform::CandidateId> {if self.comparison==ComparisonMode::Current {None}else{self.candidate.as_ref().and_then(|c|c.id)}}
    pub fn record_location(&mut self) {
        self.navigation.push(Location {revision:self.projection.revision_id,world:self.world,focus:self.focus,center:[self.camera.center.x,self.camera.center.y],zoom:self.camera.zoom});
    }
    pub fn restore_location(&mut self,location:Location) {
        // Model revisions are immutable, but may require a worker restoration.
        self.world=location.world;self.focus=location.focus;self.expanded=None;
        if let Some(binding)=self.binding && binding.revision!=location.revision {
            self.binding=Some(agq_studio_platform::RevisionBinding {revision:location.revision,..binding});
            self.request_projection();
        } else {self.rebuild();}
        self.camera.center=Point::new(location.center[0],location.center[1]);self.camera.zoom=location.zoom;self.camera_target=None;self.fit_pending=false;
    }
    pub fn request_projection(&mut self) {
        if let Some(binding)=self.binding {
            let definition=self.definition();
            self.scene_request=self.enqueue(Box::new(move|platform|platform.project(binding,&definition).map(Output::Projection)));
        } else {self.rebuild();}
    }
    pub fn switch_world(&mut self,world:World) {
        self.navigation.update_camera([self.camera.center.x,self.camera.center.y],self.camera.zoom);
        self.world=world;self.expanded=None;self.dependencies=None;
        if world!=World::System {self.focus=None;}
        if self.fixture.is_some() {
            if world==World::Requirements {self.projection=fixtures::requirements();}
            else if self.projection.view.kind==agq_modeling_view::ViewKind::Requirements {self.projection=fixture_projection(self.fixture.as_deref().unwrap_or("architecture"));}
            self.rebuild();
        } else if world!=World::History {self.request_projection();}
        self.fit_pending=true;self.record_location();
    }
    pub fn load_fixture(&mut self,name:&str) {
        self.fixture=Some(name.into());self.binding=None;self.branch=None;self.history=None;
        // Fence all outstanding read responses when entering fixture mode.
        self.scene_request=0;self.project_request=0;self.inspector_request=0;
        self.explanation_request=0;self.source_request=0;
        self.projection=fixture_projection(name);self.world=if name=="requirements" {World::Requirements}else if matches!(name,"ports"|"stress1000"|"stress10000"){World::Graph}else{World::System};
        self.focus=None;self.expanded=None;self.dependencies=None;self.candidate=None;self.compare_before=None;self.comparison=ComparisonMode::Current;self.layout=Default::default();self.selection.clear();
        self.rebuild();self.fit_pending=true;self.ready=true;
        if name=="diff" {self.show_fixture_diff();}
        self.status="Visual fixture · semantic acceptance is unavailable".into();
        self.record_location();
    }
    pub fn execute(&mut self,id:CommandId,ctx:&egui::Context) {
        if let Some(reason)=commands::unavailable(id,&self.context()) {self.status=reason.into();return;}
        use CommandId::*;
        match id {
            Theme=>{self.theme=crate::theme::Theme::new(!self.theme.dark,self.theme.contrast);self.theme.install(ctx);self.batch_key=None;}
            Contrast=>{self.theme=crate::theme::Theme::new(self.theme.dark,!self.theme.contrast);self.theme.install(ctx);self.batch_key=None;}
            ReducedMotion=>self.reduced_motion=!self.reduced_motion,
            System=>self.switch_world(World::System),Graph=>self.switch_world(World::Graph),Requirements=>self.switch_world(World::Requirements),History=>self.switch_world(World::History),
            Home=>{self.focus=None;self.expanded=None;self.dependencies=None;self.collapsed.clear();self.switch_world(World::System);}
            Fit=>self.fit_pending=true,
            Focus=>{
                self.navigation.update_camera([self.camera.center.x,self.camera.center.y],self.camera.zoom);
                if let Some(id)=self.selected_element() {
                    if self.world==World::System && self.scene.node(id).is_some_and(|n|n.is_container) {self.focus=Some(id);self.request_projection();self.fit_pending=true;}
                    else if let Some(bounds)=self.selection.primary.as_ref().and_then(|t|self.scene.target_bounds(t)) {let mut target=self.camera;target.fit(bounds,140.0);target.zoom=target.zoom.min(2.0);self.camera_target=Some(target);}
                    self.record_location();
                }
            }
            Up=>{self.focus=self.focus.and_then(|id|self.projection.nodes.iter().find(|n|n.id==id).and_then(|n|n.owner));self.request_projection();self.fit_pending=true;self.record_location();}
            Back=>{if let Some(location)=self.navigation.back(){self.restore_location(location);}}
            Forward=>{if let Some(location)=self.navigation.forward(){self.restore_location(location);}}
            Explain=>{
                self.show_explain=true;self.explanation=None;
                if let (Some(binding),Some(element))=(self.binding,self.selected_element()) {
                    let candidate=self.visible_candidate_id();
                    self.explanation_request=self.enqueue(Box::new(move|p|if let Some(id)=candidate {p.explain_candidate(id,element).map(Output::Explanation)}else{p.explain(binding,element).map(Output::Explanation)}));
                }
            }
            Source=>{
                self.show_source=true;
                if let (Some(binding),Some(element))=(self.binding,self.selected_element()) {
                    self.source_request=self.enqueue(Box::new(move|p|p.source(binding,element).map(Output::Source)));
                }
            }
            Dependencies=>{
                if let Some(id)=self.selected_element() {
                    self.world=World::Graph;self.focus=None;self.show_agent=true;
                    let mut ids=BTreeSet::from([id]);
                    for e in &self.active_projection().edges {if e.source==id || e.target==id {ids.extend([e.source,e.target]);}}
                    // Feature ports keep their owning element visible in the temporary lens.
                    if let Some(node)=self.active_projection().nodes.iter().find(|n|n.id==id) {ids.extend(node.features.iter().map(|f|f.id));}
                    for e in &self.active_projection().edges {if ids.contains(&e.source)||ids.contains(&e.target){ids.extend([e.source,e.target]);}}
                    self.dependencies=Some(ids.clone());self.expanded=None;
                    self.rebuild();self.fit_pending=true;self.status="Temporary dependency view · model unchanged".into();
                }
            }
            ExpandIncoming|ExpandOutgoing|Neighbors=>{
                if let Some(id)=self.selected_element() {
                    let mut ids=BTreeSet::from([id]);
                    for e in &self.active_projection().edges {
                        if (id==Neighbors || id==ExpandOutgoing) && e.source==self.selected_element().unwrap() {ids.insert(e.target);}
                        if (id==Neighbors || id==ExpandIncoming) && e.target==self.selected_element().unwrap() {ids.insert(e.source);}
                    }
                    if id==Neighbors {self.selection.replace(ids.into_iter().map(SceneTarget::Node));self.batch_key=None;}
                    else {self.expanded.get_or_insert_with(BTreeSet::new).extend(ids);self.rebuild();}
                }
            }
            CreatePart=>{self.create_dialog=true;self.new_part_name="newPart".into();}
            Compare=>self.compare_parent(),
            Validate=>{if let Some(id)=self.candidate.as_ref().and_then(|c|c.id) {let view=self.definition();self.enqueue(Box::new(move|p|p.validate(id,&view).map(Output::Candidate)));}}
            Commit=>{if let Some(id)=self.candidate.as_ref().and_then(|c|c.id) {self.enqueue(Box::new(move|p|p.commit(id).map(Output::Committed)));}}
            Cancel=>{
                if let Some(id)=self.candidate.as_ref().and_then(|c|c.id) {self.enqueue(Box::new(move|p|{p.cancel(id)?;Ok(Output::Cancelled)}));}
                else {self.candidate=None;self.comparison=ComparisonMode::Current;self.rebuild();}
            }
        }
    }
    pub fn prepare_part(&mut self) {
        let Some(owner)=self.selected_element() else {return;};
        let name=self.new_part_name.trim().to_owned();
        if name.is_empty() {self.status="Enter a part name".into();return;}
        if let (Some(binding),Some(branch))=(self.binding,self.branch) {
            let context=agq_modeling_agent::AgentContext {project:binding.project,branch,revision:binding.revision,selection:vec![owner]};
            let view=self.definition();
            self.enqueue(crate::bridge::nested_part(context,owner,name,view));
        } else {
            // Deterministic visual response to typed intent; no semantic reconstruction exists here.
            let _intent=agq_modeling_agent::ModelCommand::CreatePartUsage {owner,name:name.clone(),definition:None};
            let before=self.projection.clone();let mut after=before.clone();
            after.revision_id=ProjectRevisionId::from_u128(before.revision_id.as_u128()+100);
            for node in &mut after.nodes {node.revision_id=after.revision_id;}
            for edge in &mut after.edges {edge.revision_id=after.revision_id;}
            let id=ElementId::from_u128(0xfa11000000000000000000000000ffff);
            after.nodes.push(ViewNode {id,revision_id:after.revision_id,semantic_kind:"PartUsage".into(),name:name.clone(),qualified_name:None,owner:Some(owner),origin:ViewOrigin::Authored,source_available:false,features:vec![],counts:FeatureCounts::default(),badges:vec!["Visual preview".into()]});
            after.edges.push(ViewEdge {id:"fixture-candidate-ownership".into(),relationship_id:None,revision_id:after.revision_id,family:RelationshipFamily::Ownership,semantic_kind:"OwningMembership".into(),source:owner,target:id,origin:ViewOrigin::Authored,rule_id:None,label:"owns".into(),directed:true,order:0});
            self.candidate=Some(Candidate {id:None,phase:None,before,after,source:format!("VISUAL FIXTURE — illustrative intent only\npart {name};\n\nNo source reconstruction, validation or commit has occurred.")});
            self.comparison=ComparisonMode::Diff;self.rebuild();self.fit_pending=true;
            self.status="Candidate visual preview · install runtime for semantic reconstruction".into();
        }
        self.create_dialog=false;
    }
    pub fn show_fixture_diff(&mut self) {
        let (before,after)=fixtures::revision_diff();self.projection=after;self.compare_before=Some(before);self.comparison=ComparisonMode::Diff;self.rebuild();self.fit_pending=true;
    }
    pub fn compare_parent(&mut self) {
        if self.fixture.is_some() {self.show_fixture_diff();return;}
        if let Some(binding)=self.binding && let Some(parent)=self.history.as_ref().and_then(|h|h.revisions.iter().find(|r|r.revision_id==binding.revision)).and_then(|r|r.parent_revision_id) {
            let view=self.definition();self.scene_request=self.enqueue(Box::new(move|p|p.compare(binding.project,parent,binding.revision,&view).map(Output::Comparison)));
        } else {self.status="This revision has no parent".into();}
    }
    pub fn keyboard(&mut self,ctx:&egui::Context) {
        if ctx.input_mut(|i|i.consume_key(Modifiers::COMMAND,Key::K)) {self.palette=!self.palette;self.palette_focus=true;}
        if ctx.input_mut(|i|i.consume_key(Modifiers::NONE,Key::Escape)) {
            if self.palette {self.palette=false;}else if self.create_dialog {self.create_dialog=false;}else if self.show_explain||self.show_source {self.show_explain=false;self.show_source=false;}else if !self.selection.targets.is_empty(){self.selection.clear();self.inspector=None;self.batch_key=None;}else{self.execute(CommandId::Back,ctx);}return;
        }
        if ctx.wants_keyboard_input() {return;}
        for (key,modifiers,command) in [(Key::F,Modifiers::NONE,CommandId::Focus),(Key::Home,Modifiers::NONE,CommandId::Fit),(Key::D,Modifiers::NONE,CommandId::Dependencies),(Key::E,Modifiers::NONE,CommandId::Explain),(Key::N,Modifiers::NONE,CommandId::Neighbors),(Key::Num1,Modifiers::NONE,CommandId::System),(Key::Num2,Modifiers::NONE,CommandId::Graph),(Key::Num3,Modifiers::NONE,CommandId::Requirements),(Key::Num4,Modifiers::NONE,CommandId::History),(Key::ArrowLeft,Modifiers::ALT,CommandId::Back),(Key::ArrowRight,Modifiers::ALT,CommandId::Forward),(Key::ArrowUp,Modifiers::ALT,CommandId::Up)] {
            if ctx.input_mut(|i|i.consume_key(modifiers,key)) {self.execute(command,ctx);}
        }
        if ctx.input(|i|i.key_pressed(Key::ArrowDown)||i.key_pressed(Key::ArrowUp)) && !self.scene.nodes.is_empty() {
            let current=self.selected_element().and_then(|id|self.scene.nodes.iter().position(|n|n.id()==id)).unwrap_or(0);
            let next=if ctx.input(|i|i.key_pressed(Key::ArrowDown)) {(current+1)%self.scene.nodes.len()}else{(current+self.scene.nodes.len()-1)%self.scene.nodes.len()};
            self.select(SceneTarget::Node(self.scene.nodes[next].id()),false);
        }
    }
}
