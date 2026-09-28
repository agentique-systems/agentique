//! The docked Panels: the Outline, and the Inspector column (Inspector,
//! Requirements, History, Problems).
use crate::studio::Studio;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct OutlineView {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl OutlineView {
    pub fn new(studio: Entity<Studio>, _: &mut Context<Self>) -> Self {
        OutlineView { studio }
    }
}

impl Render for OutlineView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Outline")
    }
}

pub struct InspectorColumn {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl InspectorColumn {
    pub fn new(studio: Entity<Studio>, _: &mut Window, _: &mut Context<Self>) -> Self {
        InspectorColumn { studio }
    }
}

impl Render for InspectorColumn {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Inspector")
    }
}
