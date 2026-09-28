//! The Conversation with the Assistant (§3.6).
use crate::studio::Studio;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct ConversationView {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl ConversationView {
    pub fn new(studio: Entity<Studio>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _ = cx;
        ConversationView { studio }
    }
}

impl ConversationView {
    pub fn focus_input(&mut self, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for ConversationView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("ConversationView")
    }
}
