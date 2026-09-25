//! Readable review of exact revision pairs. Summary and camera are disposable;
//! canonical projections, scene coordinates, ghosts and lifecycle stay intact.
use crate::app::{ComparisonMode, StudioApp, muted};
use agq_kernel::ElementId;
use agq_modeling_view::{ViewDefinition, ViewNode, ViewOrigin, ViewProjection};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{Camera2D, DiffMark, Rect, SceneLookup, SceneTarget, SemanticScene};
use eframe::egui::{self, RichText};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone)]
struct Change {
    name: String,
    detail: String,
    mark: DiffMark,
    origin: ViewOrigin,
    revision: ProjectRevisionId,
    canonical_id: Option<ElementId>,
    target: Option<SceneTarget>,
    relationship: bool,
}

struct ChangeGroup {
    owner: Option<ElementId>,
    name: String,
    changes: Vec<Change>,
}

struct ChangeReview {
    before: ProjectRevisionId,
    after: ProjectRevisionId,
    groups: Vec<ChangeGroup>,
    objects: usize,
    relationships: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct ReviewScope {
    generation: u64,
    runtime_epoch: u64,
    binding: Option<agq_studio_platform::RevisionBinding>,
    before: ProjectRevisionId,
    after: ProjectRevisionId,
    definition: ViewDefinition,
}

#[derive(Clone)]
struct CachedReview {
    scope: ReviewScope,
    review: Arc<ChangeReview>,
}

#[derive(Clone)]
struct RememberedGroup {
    index: usize,
    selection: Option<SceneTarget>,
}

fn displayed_group(
    review: &ChangeReview,
    selection: Option<&SceneTarget>,
    remembered: Option<&RememberedGroup>,
) -> usize {
    if let Some(remembered) = remembered
        && remembered.selection.as_ref() == selection
    {
        return remembered.index.min(review.groups.len().saturating_sub(1));
    }
    review
        .groups
        .iter()
        .position(|group| {
            group
                .changes
                .iter()
                .any(|change| same_target(change.target.as_ref(), selection))
        })
        .or_else(|| remembered.map(|remembered| remembered.index))
        .unwrap_or(0)
        .min(review.groups.len().saturating_sub(1))
}

fn mark_label(mark: DiffMark) -> &'static str {
    match mark {
        DiffMark::Added => "+ Added",
        DiffMark::Removed => "− Removed",
        DiffMark::Changed => "~ Changed",
        DiffMark::Unchanged => "Unchanged",
    }
}

fn origin_label(origin: ViewOrigin) -> &'static str {
    match origin {
        ViewOrigin::Authored => "Authored",
        ViewOrigin::Derived => "Derived",
        ViewOrigin::Standard => "Standard library",
        ViewOrigin::Generated => "Generated",
    }
}

fn change_rank(change: &Change) -> (u8, bool) {
    let origin = match change.origin {
        ViewOrigin::Authored => 0,
        ViewOrigin::Derived => 1,
        ViewOrigin::Generated => 2,
        ViewOrigin::Standard => 3,
    };
    (origin, change.relationship)
}

fn same_target(a: Option<&SceneTarget>, b: Option<&SceneTarget>) -> bool {
    match (a, b) {
        (
            Some(SceneTarget::Node(a) | SceneTarget::Container(a)),
            Some(SceneTarget::Node(b) | SceneTarget::Container(b)),
        ) => a == b,
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn node_changed(before: &ViewNode, after: &ViewNode) -> bool {
    let mut before = before.clone();
    before.revision_id = after.revision_id;
    before != *after
}

/// A summary never compares lenses, combines projects by label, or reads marks
/// from a differently bound scene. Its counts describe the supplied view pair.
fn change_review(
    before: &ViewProjection,
    after: &ViewProjection,
    scene: &SemanticScene,
) -> Option<ChangeReview> {
    if before.view != after.view || scene.revision_id != after.revision_id {
        return None;
    }
    let before_nodes: BTreeMap<_, _> = before.nodes.iter().map(|node| (node.id, node)).collect();
    let after_nodes: BTreeMap<_, _> = after.nodes.iter().map(|node| (node.id, node)).collect();
    let lookup = SceneLookup::build(scene);
    let mut groups: BTreeMap<(Option<ElementId>, bool), ChangeGroup> = BTreeMap::new();
    let mut add = |owner: Option<ElementId>,
                   context: &ViewProjection,
                   unknown_owner: bool,
                   change: Change| {
        let nodes = if context.revision_id == after.revision_id {
            &after_nodes
        } else {
            &before_nodes
        };
        let owner_name = owner
            .and_then(|id| nodes.get(&id))
            .map(|node| node.name.clone())
            .unwrap_or_else(|| {
                if unknown_owner {
                    "Source outside this view".into()
                } else if owner.is_some() {
                    "Owner outside this view".into()
                } else {
                    "Top-level architecture".into()
                }
            });
        groups
            .entry((owner, unknown_owner))
            .or_insert_with(|| ChangeGroup {
                owner,
                name: owner_name,
                changes: vec![],
            })
            .changes
            .push(change);
    };
    let mut objects = 0;
    for id in before_nodes
        .keys()
        .chain(after_nodes.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let old = before_nodes.get(&id).copied();
        let new = after_nodes.get(&id).copied();
        let mark = match (old, new) {
            (None, Some(_)) => DiffMark::Added,
            (Some(_), None) => DiffMark::Removed,
            (Some(old), Some(new)) if node_changed(old, new) => DiffMark::Changed,
            _ => continue,
        };
        let node = new.or(old)?;
        let context = if new.is_some() { after } else { before };
        let target = [SceneTarget::Port(id), SceneTarget::Node(id)]
            .into_iter()
            .find(|target| target_revision(scene, &lookup, target) == Some(node.revision_id));
        let mut details = vec![origin_label(node.origin).to_owned()];
        if let (Some(old), Some(new)) = (old, new) {
            if old.name != new.name {
                details.push(format!("Renamed from {}", old.name));
            }
            if old.owner != new.owner {
                let owner_name = |owner: Option<ElementId>, view: &ViewProjection| {
                    let nodes = if view.revision_id == after.revision_id {
                        &after_nodes
                    } else {
                        &before_nodes
                    };
                    owner
                        .and_then(|id| nodes.get(&id))
                        .map_or("outside this view", |node| node.name.as_str())
                        .to_owned()
                };
                details.push(format!(
                    "Owner: {} → {}",
                    owner_name(old.owner, before),
                    owner_name(new.owner, after)
                ));
            }
        }
        add(
            node.owner,
            context,
            false,
            Change {
                name: node.name.clone(),
                detail: details.join(" · "),
                mark,
                origin: node.origin,
                revision: node.revision_id,
                canonical_id: Some(id),
                target,
                relationship: false,
            },
        );
        objects += 1;
    }
    let old_edges: BTreeMap<_, _> = before.edges.iter().map(|edge| (&edge.id, edge)).collect();
    let new_edges: BTreeMap<_, _> = after.edges.iter().map(|edge| (&edge.id, edge)).collect();
    let mut relationships = 0;
    for id in old_edges
        .keys()
        .chain(new_edges.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let old = old_edges.get(id).copied();
        let new = new_edges.get(id).copied();
        let mark = match (old, new) {
            (None, Some(_)) => DiffMark::Added,
            (Some(_), None) => DiffMark::Removed,
            (Some(old), Some(new)) => {
                let mut same_revision = old.clone();
                same_revision.revision_id = new.revision_id;
                if same_revision == *new {
                    continue;
                }
                DiffMark::Changed
            }
            _ => continue,
        };
        let edge = new.or(old)?;
        let context = if new.is_some() { after } else { before };
        let nodes = if new.is_some() {
            &after_nodes
        } else {
            &before_nodes
        };
        let source = nodes.get(&edge.source).copied();
        let target = nodes.get(&edge.target).copied();
        // Group relationships by the source's recorded owner, never by a
        // fabricated connector owner or a layout-inferred semantic direction.
        let owner = source.and_then(|node| node.owner);
        let target_key = SceneTarget::Edge(edge.id.clone());
        add(
            owner,
            context,
            source.is_none(),
            Change {
                name: format!(
                    "{} {} {}",
                    source.map_or("Endpoint outside view", |node| &node.name),
                    edge.label,
                    target.map_or("endpoint outside view", |node| &node.name)
                ),
                detail: format!(
                    "{} · {:?} · source owner context",
                    origin_label(edge.origin),
                    edge.family
                ),
                mark,
                origin: edge.origin,
                revision: edge.revision_id,
                canonical_id: edge.relationship_id,
                target: (target_revision(scene, &lookup, &target_key) == Some(edge.revision_id))
                    .then_some(target_key),
                relationship: true,
            },
        );
        relationships += 1;
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    for group in &mut groups {
        group.changes.sort_by(|a, b| {
            change_rank(a)
                .cmp(&change_rank(b))
                .then(a.name.cmp(&b.name))
        });
    }
    groups.sort_by(|a, b| {
        change_rank(&a.changes[0])
            .cmp(&change_rank(&b.changes[0]))
            .then_with(|| {
                b.changes
                    .iter()
                    .filter(|change| !change.relationship)
                    .count()
                    .cmp(
                        &a.changes
                            .iter()
                            .filter(|change| !change.relationship)
                            .count(),
                    )
            })
            .then(a.name.cmp(&b.name))
            .then(a.owner.cmp(&b.owner))
    });
    Some(ChangeReview {
        before: before.revision_id,
        after: after.revision_id,
        groups,
        objects,
        relationships,
    })
}

fn target_revision(
    scene: &SemanticScene,
    lookup: &SceneLookup,
    target: &SceneTarget,
) -> Option<ProjectRevisionId> {
    match target {
        SceneTarget::Node(id) | SceneTarget::Container(id) => lookup
            .node(scene, *id)
            .map(|node| node.semantic.revision_id),
        SceneTarget::Port(id) => lookup.port(scene, *id).map(|port| port.revision_id),
        SceneTarget::Edge(id) => lookup.edge(scene, id).map(|edge| edge.semantic.revision_id),
    }
}

fn object_bounds(scene: &SemanticScene, lookup: &SceneLookup, id: ElementId) -> Option<Rect> {
    if let Some(node) = lookup.node(scene, id) {
        return Some(if node.is_container {
            // Frame a legible owner header, not its potentially enormous envelope.
            Rect::new(
                node.bounds.min.x,
                node.bounds.min.y,
                node.bounds.width().min(320.0),
                72.0,
            )
        } else {
            node.bounds
        });
    }
    lookup
        .port(scene, id)
        .map(|port| Rect::new(port.position.x - 7.0, port.position.y - 7.0, 14.0, 14.0))
}

fn change_bounds(scene: &SemanticScene, lookup: &SceneLookup, change: &Change) -> Vec<Rect> {
    let Some(target) = &change.target else {
        return vec![];
    };
    if target_revision(scene, lookup, target) != Some(change.revision) {
        return vec![];
    }
    match target {
        SceneTarget::Node(id) | SceneTarget::Container(id) => {
            object_bounds(scene, lookup, *id).into_iter().collect()
        }
        SceneTarget::Port(id) => object_bounds(scene, lookup, *id)
            .into_iter()
            .chain(
                lookup
                    .port(scene, *id)
                    .and_then(|port| object_bounds(scene, lookup, port.owner)),
            )
            .collect(),
        SceneTarget::Edge(id) => lookup
            .edge(scene, id)
            .map(|edge| {
                [edge.semantic.source, edge.semantic.target]
                    .into_iter()
                    .filter_map(|id| object_bounds(scene, lookup, id))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

struct ChangeFocus {
    camera: Camera2D,
    nearby: usize,
    visible: usize,
}

fn focus_group(
    scene: &SemanticScene,
    group: &ChangeGroup,
    camera: Camera2D,
    primary: Option<&SceneTarget>,
) -> Option<ChangeFocus> {
    let lookup = SceneLookup::build(scene);
    let mut entries: Vec<_> = group
        .changes
        .iter()
        .filter_map(|change| {
            let bounds = change_bounds(scene, &lookup, change);
            (!bounds.is_empty()).then_some((change, bounds))
        })
        .collect();
    entries.sort_by_key(|(change, _)| usize::from(!same_target(change.target.as_ref(), primary)));
    let mut bounds = *entries.first()?.1.first()?;
    let mut nearby = 0;
    for (_, anchors) in &entries {
        let mut included = false;
        for anchor in anchors {
            let proposed = bounds.union(*anchor);
            let mut fitted = camera;
            fitted.fit(proposed, 72.0);
            if fitted.zoom >= 0.42 {
                bounds = proposed;
                included = true;
            }
        }
        nearby += usize::from(included);
    }
    if let Some(owner) = group.owner.and_then(|id| object_bounds(scene, &lookup, id)) {
        let mut fitted = camera;
        fitted.fit(bounds.union(owner), 72.0);
        if fitted.zoom >= 0.42 {
            bounds = bounds.union(owner);
        }
    }
    let mut target = camera;
    target.fit(bounds, 72.0);
    target.zoom = target.zoom.min(1.2);
    Some(ChangeFocus {
        camera: target,
        nearby,
        visible: entries.len(),
    })
}

impl StudioApp {
    fn change_pair(&self) -> Option<(&ViewProjection, &ViewProjection)> {
        if self.comparison != ComparisonMode::Diff {
            return None;
        }
        let (before, after) = if let Some(candidate) = &self.candidate {
            (&candidate.before, &candidate.after)
        } else {
            (self.compare_before.as_ref()?, &self.projection)
        };
        (before.view == after.view && self.scene.revision_id == after.revision_id)
            .then_some((before, after))
    }

    fn change_review(&self) -> Option<ChangeReview> {
        let (before, after) = self.change_pair()?;
        change_review(before, after, &self.scene)
    }

    fn cached_change_review(&self, ui: &egui::Ui) -> Option<Arc<ChangeReview>> {
        let (before, after) = self.change_pair()?;
        let scope = ReviewScope {
            generation: self.generation,
            runtime_epoch: self.bridge.epoch(),
            binding: self.binding,
            before: before.revision_id,
            after: after.revision_id,
            definition: after.view.clone(),
        };
        let key = ui.id().with("exact-change-review-cache");
        if let Some(cached) = ui.ctx().data(|data| data.get_temp::<CachedReview>(key))
            && cached.scope == scope
        {
            return Some(cached.review);
        }
        let review = Arc::new(change_review(before, after, &self.scene)?);
        // One replacement slot, not an accumulating per-revision cache. Scene
        // generation changes only after a successful complete scene install.
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                key,
                CachedReview {
                    scope,
                    review: review.clone(),
                },
            )
        });
        Some(review)
    }

    /// Focus readable local change context without rebuilding, filtering,
    /// selecting a new revision, or modifying the current semantic selection.
    pub fn focus_changes(&mut self) {
        let Some(review) = self.change_review() else {
            return;
        };
        let group = review
            .groups
            .iter()
            .find(|group| {
                group.changes.iter().any(|change| {
                    same_target(change.target.as_ref(), self.selection.primary.as_ref())
                })
            })
            .or_else(|| {
                review.groups.iter().find(|group| {
                    group.owner.is_some()
                        && group.owner == self.selected_element()
                        && group.changes.iter().any(|change| change.target.is_some())
                })
            })
            .or_else(|| {
                review
                    .groups
                    .iter()
                    .find(|group| group.changes.iter().any(|change| change.target.is_some()))
            });
        if let Some(group) = group {
            self.focus_change_group(group);
        } else {
            self.status =
                "No changed objects are visible in this view; the exact comparison is retained"
                    .into();
        }
    }

    fn focus_change_group(&mut self, group: &ChangeGroup) {
        let Some(focus) = focus_group(
            &self.scene,
            group,
            self.camera,
            self.selection.primary.as_ref(),
        ) else {
            return;
        };
        self.fit_pending = false;
        if self.reduced_motion {
            self.camera = focus.camera;
            self.camera_target = None;
        } else {
            self.camera_target = Some(focus.camera);
        }
        self.status = format!(
            "{} · nearby context for {} of {} visible changes; full comparison retained",
            group.name, focus.nearby, focus.visible
        );
    }

    pub fn diff_review_panel(&mut self, ui: &mut egui::Ui) {
        let Some(review) = self.cached_change_review(ui) else {
            return;
        };
        let theme = self.theme;
        egui::Frame::new().fill(theme.surface).inner_margin(10.0).corner_radius(6.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Design changes");
                ui.label(muted(format!("{} objects · {} relationships in this view", review.objects, review.relationships), theme).small());
                if ui.small_button("Fit entire comparison").clicked() { self.execute(crate::commands::CommandId::Fit, ui.ctx()); }
            });
            if review.groups.is_empty() { ui.label("No projected changes in this comparison."); return; }
            let key = ui.id().with(("change-review-group", self.generation, review.before.to_string(), review.after.to_string()));
            let remembered = ui.ctx().data(|data| data.get_temp::<RememberedGroup>(key));
            let mut index = displayed_group(&review, self.selection.primary.as_ref(), remembered.as_ref());
            let previous = index;
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt(key).selected_text(&review.groups[index].name).show_ui(ui, |ui| {
                    for (i, group) in review.groups.iter().enumerate() {
                        ui.selectable_value(&mut index, i, format!("{} · {} changes", group.name, group.changes.len()));
                    }
                });
                if ui.small_button("Focus this group").clicked() { self.focus_change_group(&review.groups[index]); }
                ui.label(muted(format!("{} owner groups", review.groups.len()), theme).small());
            });
            if index != previous {
                if let Some(target) = review.groups[index].changes.iter().find_map(|change| change.target.clone()) { self.select(target, false); }
                self.focus_change_group(&review.groups[index]);
            }
            ui.ctx().data_mut(|data| data.insert_temp(key, RememberedGroup { index, selection: self.selection.primary.clone() }));
            let group = &review.groups[index];
            for change in group.changes.iter().take(3) { self.change_row(ui, change); }
            if group.changes.len() > 3 {
                ui.collapsing(format!("All {} changes in {}", group.changes.len(), group.name), |ui| {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for change in &group.changes { self.change_row(ui, change); }
                    });
                });
            }
            ui.label(muted("Focus keeps existing positions. Distant changes remain in the list and the full comparison.", theme).small());
        });
    }

    fn change_row(&mut self, ui: &mut egui::Ui, change: &Change) {
        let color = match change.mark {
            DiffMark::Added => self.theme.green,
            DiffMark::Removed => self.theme.amber,
            _ => self.theme.accent,
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(mark_label(change.mark)).small().color(color));
            let response = ui.add_enabled(
                change.target.is_some(),
                egui::Button::new(&change.name).frame(false).wrap(),
            );
            if response.clicked()
                && let Some(target) = &change.target
            {
                self.select(target.clone(), false);
                self.focus_changes();
            }
            response.on_hover_text(format!(
                "{}\nRevision {}\nCanonical identity {}{}",
                change.detail,
                change.revision,
                change.canonical_id.map_or_else(
                    || "No element identity in this projection".into(),
                    |id| id.to_string()
                ),
                if change.target.is_none() {
                    "\nOutside the currently displayed scene"
                } else {
                    ""
                }
            ));
            ui.label(muted(&change.detail, self.theme).small());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_studio_scene::{SceneOptions, Size, fixtures};

    fn pair() -> (ViewProjection, ViewProjection) {
        let (before, mut after) = fixtures::revision_diff();
        // The explicit visual fixture has different cosmetic view names. These
        // test DTOs deliberately describe one exact query on two revisions.
        after.view = before.view.clone();
        (before, after)
    }

    fn scene(before: &ViewProjection, after: &ViewProjection) -> SemanticScene {
        let options = SceneOptions {
            hierarchy: false,
            ..Default::default()
        };
        let mut scene = SemanticScene::from_projection(after, &options, None).unwrap();
        let parent = SemanticScene::from_projection(before, &options, None).unwrap();
        scene.apply_diff(&parent);
        scene
    }

    fn next_revision(before: &ViewProjection) -> ViewProjection {
        let mut after = before.clone();
        after.revision_id = ProjectRevisionId::from_u128(99);
        for node in &mut after.nodes {
            node.revision_id = after.revision_id;
        }
        for edge in &mut after.edges {
            edge.revision_id = after.revision_id;
        }
        after
    }

    #[test]
    fn change_review_retains_exact_ghost_revision_and_rejects_mixed_lenses() {
        let (before, after) = pair();
        let scene = scene(&before, &after);
        let review = change_review(&before, &after, &scene).unwrap();
        let removed = review
            .groups
            .iter()
            .flat_map(|group| &group.changes)
            .find(|change| change.canonical_id == Some(fixtures::id(13)))
            .unwrap();
        assert_eq!(removed.mark, DiffMark::Removed);
        assert_eq!(removed.revision, before.revision_id);
        assert_eq!(removed.target, Some(SceneTarget::Node(fixtures::id(13))));
        assert_eq!(
            scene.target_revision(removed.target.as_ref().unwrap()),
            Some(before.revision_id)
        );
        let renamed = review
            .groups
            .iter()
            .flat_map(|group| &group.changes)
            .find(|change| change.canonical_id == Some(fixtures::id(22)))
            .unwrap();
        assert!(renamed.detail.contains("Renamed from ModelingService"));
        let mut other_lens = before.clone();
        other_lens.view.depth += 1;
        assert!(change_review(&other_lens, &after, &scene).is_none());
        let mut other_scene = scene.clone();
        other_scene.revision_id = before.revision_id;
        assert!(change_review(&before, &after, &other_scene).is_none());
    }

    #[test]
    fn explicit_hidden_group_choice_survives_until_semantic_selection_changes() {
        let (before, after) = pair();
        let scene = scene(&before, &after);
        let mut review = change_review(&before, &after, &scene).unwrap();
        assert!(review.groups.len() > 1);
        let old_selection = review.groups[0]
            .changes
            .iter()
            .find_map(|change| change.target.clone())
            .unwrap();
        // Full-pair review retains this group although presentation excludes
        // all its targets. Choosing it must not force an invalid scene select.
        let hidden = review.groups.len() - 1;
        for change in &mut review.groups[hidden].changes {
            change.target = None;
        }
        let remembered = RememberedGroup {
            index: hidden,
            selection: Some(old_selection.clone()),
        };
        assert_eq!(
            displayed_group(&review, Some(&old_selection), Some(&remembered)),
            hidden
        );
        let changed_selection = review
            .groups
            .iter()
            .enumerate()
            .take(hidden)
            .flat_map(|(index, group)| {
                group
                    .changes
                    .iter()
                    .filter_map(move |change| change.target.as_ref().map(|target| (index, target)))
            })
            .find(|(_, target)| **target != old_selection)
            .unwrap();
        assert_eq!(
            displayed_group(&review, Some(changed_selection.1), Some(&remembered)),
            changed_selection.0
        );
        let no_selection = RememberedGroup {
            index: hidden,
            selection: None,
        };
        assert_eq!(displayed_group(&review, None, Some(&no_selection)), hidden);
    }

    #[test]
    fn port_only_and_relationship_only_changes_have_real_focus_targets() {
        let before = fixtures::architecture();
        let mut after = next_revision(&before);
        let mut port = after
            .nodes
            .iter()
            .find(|node| node.id == fixtures::id(21))
            .unwrap()
            .clone();
        port.id = fixtures::id(901);
        port.name = "newReadInterface".into();
        port.semantic_kind = "PortUsage".into();
        port.owner = Some(fixtures::id(21));
        port.features.clear();
        port.counts = Default::default();
        after.nodes.push(port);
        let port_scene = scene(&before, &after);
        let review = change_review(&before, &after, &port_scene).unwrap();
        assert_eq!(review.objects, 1);
        assert_eq!(review.relationships, 0);
        assert_eq!(review.groups[0].owner, Some(fixtures::id(21)));
        let change = &review.groups[0].changes[0];
        assert_eq!(change.target, Some(SceneTarget::Port(fixtures::id(901))));
        assert_eq!(change.revision, after.revision_id);
        assert!(focus_group(&port_scene, &review.groups[0], Camera2D::default(), None).is_some());

        let mut after = next_revision(&before);
        let edge = after
            .edges
            .iter_mut()
            .find(|edge| edge.source == fixtures::id(21) && edge.target == fixtures::id(23))
            .unwrap();
        edge.label = "updated recorded relation label".into();
        let edge_id = edge.id.clone();
        let edge_scene = scene(&before, &after);
        let review = change_review(&before, &after, &edge_scene).unwrap();
        assert_eq!(review.objects, 0);
        assert_eq!(review.relationships, 1);
        assert_eq!(
            review.groups[0].changes[0].target,
            Some(SceneTarget::Edge(edge_id))
        );
        let mut stale = review.groups[0].changes[0].clone();
        stale.revision = before.revision_id;
        assert!(change_bounds(&edge_scene, &SceneLookup::build(&edge_scene), &stale).is_empty());
        assert!(focus_group(&edge_scene, &review.groups[0], Camera2D::default(), None).is_some());
    }

    #[test]
    fn dispersed_changes_focus_locally_without_moving_any_scene_object() {
        let (before, mut after) = pair();
        for node in &mut after.nodes {
            if [fixtures::id(21), fixtures::id(23)].contains(&node.id) {
                node.name.push_str("Updated");
            }
        }
        let mut scene = scene(&before, &after);
        for (n, id) in [21, 22, 23, 24].into_iter().enumerate() {
            scene
                .nodes
                .iter_mut()
                .find(|node| node.id() == fixtures::id(id))
                .unwrap()
                .bounds = Rect::new(n as f32 * 6_000.0, 300.0, 232.0, 118.0);
        }
        let coordinates: Vec<_> = scene
            .nodes
            .iter()
            .map(|node| (node.id(), node.bounds))
            .collect();
        let review = change_review(&before, &after, &scene).unwrap();
        let group = review
            .groups
            .iter()
            .find(|group| group.owner == Some(fixtures::id(2)))
            .unwrap();
        let selection = SceneTarget::Node(fixtures::id(22));
        let camera = Camera2D {
            viewport: Size::new(1000.0, 500.0),
            ..Default::default()
        };
        let focused = focus_group(&scene, group, camera, Some(&selection)).unwrap();
        assert!(focused.camera.zoom >= 0.42);
        assert!(
            focused.nearby < focused.visible,
            "distant changes must not shrink the selected object into an unreadable union"
        );
        assert!(
            focused
                .camera
                .visible_rect()
                .contains(scene.node(fixtures::id(22)).unwrap().bounds.center())
        );
        assert_eq!(
            coordinates,
            scene
                .nodes
                .iter()
                .map(|node| (node.id(), node.bounds))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            scene.target_revision(&SceneTarget::Node(fixtures::id(13))),
            Some(before.revision_id)
        );
    }

    #[test]
    fn native_change_focus_keeps_candidate_phase_binding_and_ghost_selection() {
        use clap::Parser;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "studio",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let ctx = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(&ctx, args).unwrap();
        let (before, after) = pair();
        app.binding = Some(agq_studio_platform::RevisionBinding {
            project: agq_modeling_repository::ProjectId::new(),
            revision: before.revision_id,
        });
        app.projection = before.clone();
        app.scene = scene(&before, &after);
        app.selection.reconcile(&app.scene);
        app.selection
            .select(SceneTarget::Node(fixtures::id(13)), false);
        app.candidate = Some(crate::app::Candidate {
            id: None,
            phase: Some(agq_studio_platform::CandidatePhase::Working),
            before: before.clone(),
            after: after.clone(),
            intent: "Test candidate".into(),
            actor: "test".into(),
            source: String::new(),
            review_selection: None,
        });
        app.reduced_motion = true;
        let binding = app.binding;
        let selection = app.selection.primary.clone();
        let old_camera = app.camera;
        app.comparison = ComparisonMode::Current;
        app.focus_changes();
        assert_eq!(app.camera, old_camera);
        app.comparison = ComparisonMode::Candidate;
        app.focus_changes();
        assert_eq!(app.camera, old_camera);
        app.comparison = ComparisonMode::Diff;
        app.focus_changes();
        assert_eq!(app.binding, binding);
        assert_eq!(app.selection.primary, selection);
        assert_eq!(
            app.selected_context().unwrap().0.revision,
            before.revision_id
        );
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(agq_studio_platform::CandidatePhase::Working)
        );
        assert_eq!(app.candidate.as_ref().unwrap().before, before);
        assert_eq!(app.candidate.as_ref().unwrap().after, after);
        assert_eq!(app.comparison, ComparisonMode::Diff);
    }
}
