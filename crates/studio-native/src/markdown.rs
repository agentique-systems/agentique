//! Markdown in the Conversation: a small renderer on `pulldown-cmark` that
//! turns a message into egui text layouts (paragraphs, headings, bold and
//! italic with real font weights, inline code, fenced code blocks with a copy
//! button, lists and quotes). An inline code span that names an element of
//! the model is a link: clicking it selects the element.
//!
//! [`parse`] runs once per message text (the Conversation caches the blocks);
//! [`show`] lays them out at the panel's width, and egui reuses the layout
//! while the text and width stay the same.
use crate::targets::{Target, record};
use crate::theme::{self, Theme};
use agq_language::ElementId;
use eframe::egui::{
    self, Color32, CornerRadius, Sense, Stroke, Vec2,
    text::{CCursor, LayoutJob, TextFormat},
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// One block of a message.
pub enum Block {
    Text(TextBlock),
    Code { language: String, code: String },
    Rule,
}

/// A paragraph, heading or list item.
pub struct TextBlock {
    /// The text with its formats; the wrap width is set when it is shown.
    pub job: LayoutJob,
    /// Element links, as character ranges of the text.
    pub links: Vec<(Range<usize>, ElementId)>,
    pub indent: f32,
    /// A list item's bullet or number.
    pub marker: Option<String>,
    pub quote: bool,
}

const LIST_INDENT: f32 = 18.0;
const QUOTE_INDENT: f32 = 12.0;

/// Parses `text` into blocks. `resolve` names the element an inline code
/// span refers to, if any.
pub fn parse(text: &str, theme: Theme, resolve: &dyn Fn(&str) -> Option<ElementId>) -> Vec<Block> {
    let mut builder = Builder {
        theme,
        resolve,
        blocks: Vec::new(),
        current: None,
        marker: None,
        bold: 0,
        italic: 0,
        strike: 0,
        link: 0,
        heading: None,
        lists: Vec::new(),
        quote: 0,
        code: None,
    };
    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH) {
        builder.event(event);
    }
    builder.flush();
    builder.blocks
}

struct Builder<'a> {
    theme: Theme,
    resolve: &'a dyn Fn(&str) -> Option<ElementId>,
    blocks: Vec<Block>,
    current: Option<TextBlock>,
    /// The marker for the next text block (a list item just started).
    marker: Option<String>,
    bold: usize,
    italic: usize,
    strike: usize,
    link: usize,
    heading: Option<HeadingLevel>,
    /// Open lists: the next number of an ordered list.
    lists: Vec<Option<u64>>,
    quote: usize,
    code: Option<(String, String)>,
}

impl Builder<'_> {
    fn event(&mut self, event: Event) {
        if let Some((_, code)) = &mut self.code {
            match event {
                Event::Text(text) => code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let (language, mut code) = self.code.take().expect("inside a code block");
                    if code.ends_with('\n') {
                        code.pop();
                    }
                    self.blocks.push(Block::Code { language, code });
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    self.flush();
                    self.heading = Some(level);
                }
                Tag::List(start) => {
                    self.flush();
                    self.lists.push(start);
                }
                Tag::Item => {
                    self.flush();
                    self.marker = Some(match self.lists.last_mut() {
                        Some(Some(number)) => {
                            *number += 1;
                            format!("{}.", *number - 1)
                        }
                        _ => "•".to_string(),
                    });
                }
                Tag::CodeBlock(kind) => {
                    self.flush();
                    let language = match kind {
                        CodeBlockKind::Fenced(language) => language.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    self.code = Some((language, String::new()));
                }
                Tag::BlockQuote(_) => {
                    self.flush();
                    self.quote += 1;
                }
                Tag::Strong => self.bold += 1,
                Tag::Emphasis => self.italic += 1,
                Tag::Strikethrough => self.strike += 1,
                Tag::Link { .. } => self.link += 1,
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Item => self.flush(),
                TagEnd::Heading(_) => {
                    self.flush();
                    self.heading = None;
                }
                TagEnd::List(_) => {
                    self.flush();
                    self.lists.pop();
                }
                TagEnd::BlockQuote(_) => {
                    self.flush();
                    self.quote = self.quote.saturating_sub(1);
                }
                TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
                TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
                TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
                TagEnd::Link => self.link = self.link.saturating_sub(1),
                _ => {}
            },
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => self.text(&text),
            Event::Code(code) => self.code_span(&code),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.text("\n"),
            Event::Rule => {
                self.flush();
                self.blocks.push(Block::Rule);
            }
            _ => {}
        }
    }

    fn flush(&mut self) {
        if let Some(block) = self.current.take()
            && !block.job.text.trim().is_empty()
        {
            self.blocks.push(Block::Text(block));
        }
    }

    fn block(&mut self) -> &mut TextBlock {
        let indent = LIST_INDENT * self.lists.len() as f32 + QUOTE_INDENT * self.quote as f32;
        let (marker, quote) = (&mut self.marker, self.quote > 0);
        self.current.get_or_insert_with(|| TextBlock {
            job: LayoutJob::default(),
            links: Vec::new(),
            indent,
            marker: marker.take(),
            quote,
        })
    }

    fn size(&self) -> f32 {
        match self.heading {
            Some(HeadingLevel::H1) => theme::HEADING + 1.5,
            Some(HeadingLevel::H2) => theme::HEADING,
            _ => theme::BODY,
        }
    }

    fn format(&self) -> TextFormat {
        let size = self.size();
        let theme = self.theme;
        let font = if self.heading.is_some() || self.bold > 0 {
            theme::semibold(size)
        } else {
            theme::regular(size)
        };
        let color = if self.link > 0 {
            theme.accent
        } else if self.quote > 0 {
            theme.text_secondary
        } else {
            theme.text
        };
        TextFormat {
            font_id: font,
            line_height: Some(size * 1.45),
            color,
            italics: self.italic > 0,
            strikethrough: if self.strike > 0 {
                Stroke::new(1.0, color)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        }
    }

    fn text(&mut self, text: &str) {
        let format = self.format();
        self.block().job.append(text, 0.0, format);
    }

    fn code_span(&mut self, code: &str) {
        let theme = self.theme;
        let mut format = TextFormat {
            font_id: theme::code(theme::CODE - 0.5),
            background: theme.elevated,
            ..self.format()
        };
        let element = (self.resolve)(code.trim());
        if element.is_some() {
            format.color = theme.accent;
            format.underline = Stroke::new(1.0, theme.accent.gamma_multiply(0.5));
        }
        let block = self.block();
        let start = block.job.text.chars().count();
        block.job.append(code, 0.0, format);
        if let Some(id) = element {
            let end = start + code.chars().count();
            block.links.push((start..end, id));
        }
    }
}

/// Shows the blocks at the available width. Returns the element whose link
/// was clicked. `id` names the message, so its links keep their identity
/// when other messages above it change.
pub fn show(ui: &mut egui::Ui, id: egui::Id, blocks: &[Block], theme: Theme) -> Option<ElementId> {
    let width = ui.available_width();
    let mut clicked = None;
    let mut previous_item = false;
    for (index, block) in blocks.iter().enumerate() {
        let item = matches!(block, Block::Text(text) if text.marker.is_some());
        if index > 0 {
            ui.add_space(if item && previous_item {
                theme::SPACE_S
            } else {
                theme::SPACE
            });
        }
        previous_item = item;
        match block {
            Block::Text(text) => {
                if let Some(element) = text_block(ui, id.with(index), text, width, theme) {
                    clicked = Some(element);
                }
            }
            Block::Code { language, code } => code_block(ui, language, code, width, theme),
            Block::Rule => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 9.0), Sense::hover());
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(theme::HAIRLINE, theme.border),
                );
            }
        }
    }
    clicked
}

fn text_block(
    ui: &mut egui::Ui,
    id: egui::Id,
    block: &TextBlock,
    width: f32,
    theme: Theme,
) -> Option<ElementId> {
    let mut job = block.job.clone();
    job.wrap.max_width = (width - block.indent).max(40.0);
    let galley = ui.painter().layout_job(job);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());
    let response = if block.links.is_empty() {
        response
    } else {
        ui.interact(rect, id, Sense::click())
    };
    let origin = rect.min + Vec2::new(block.indent, 0.0);

    if let Some(marker) = &block.marker {
        let font = block
            .job
            .sections
            .first()
            .map_or(theme::regular(theme::BODY), |s| s.format.font_id.clone());
        let row = galley.rows.first().map_or(0.0, |row| row.rect().center().y);
        ui.painter().text(
            egui::pos2(origin.x - 6.0, origin.y + row),
            egui::Align2::RIGHT_CENTER,
            marker,
            font,
            theme.muted,
        );
    }
    if block.quote {
        ui.painter().vline(
            rect.left() + 2.0,
            rect.y_range(),
            Stroke::new(2.0, theme.border_strong),
        );
    }
    let mut clicked = None;
    for (range, id) in &block.links {
        // The start of the link, for the scripted journeys.
        let at = galley.pos_from_cursor(CCursor::new(range.start));
        let start = egui::Rect::from_min_size(at.min, Vec2::new(6.0, at.height()));
        record(
            ui.ctx(),
            Target::Link(id.raw()),
            start.translate(origin.to_vec2()),
        );
    }
    if !block.links.is_empty()
        && let Some(pointer) = response.hover_pos()
    {
        let cursor = galley.cursor_from_pos(pointer - origin);
        let row_hit = galley.rect.translate(origin.to_vec2()).contains(pointer);
        if let Some((_, id)) = block
            .links
            .iter()
            .find(|(range, _)| row_hit && range.start <= cursor.index && cursor.index <= range.end)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if response.clicked() {
                clicked = Some(*id);
            }
        }
    }
    ui.painter().galley(origin, galley, theme.text);
    clicked
}

fn code_block(ui: &mut egui::Ui, language: &str, code: &str, width: f32, theme: Theme) {
    let padding = theme::SPACE;
    let mut job = LayoutJob::simple(
        code.to_string(),
        theme::code(theme::CODE - 0.5),
        theme.text,
        width - 2.0 * padding,
    );
    job.wrap.break_anywhere = false;
    let galley = ui.painter().layout_job(job);
    let header = 20.0;
    let size = Vec2::new(width, galley.size().y + header + padding);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let fill = if theme.dark {
        theme.canvas
    } else {
        theme.canvas.lerp_to_gamma(Color32::WHITE, 0.3)
    };
    ui.painter().rect(
        rect,
        CornerRadius::from(theme::RADIUS),
        fill,
        Stroke::new(theme::HAIRLINE, theme.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.min + Vec2::new(padding, header / 2.0 + 2.0),
        egui::Align2::LEFT_CENTER,
        if language.is_empty() {
            "code"
        } else {
            language
        },
        theme::regular(theme::CAPTION),
        theme.muted,
    );
    let copy = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 50.0, rect.top() + 2.0),
        Vec2::new(46.0, header),
    );
    let response = ui.put(
        copy,
        egui::Button::new(egui::RichText::new("Copy").font(theme::regular(theme::CAPTION)))
            .frame(false),
    );
    if response.clicked() {
        ui.ctx().copy_text(code.to_string());
    }
    ui.painter().galley(
        rect.min + Vec2::new(padding, header + 2.0),
        galley,
        theme.text,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(block: &Block) -> &str {
        match block {
            Block::Text(text) => &text.job.text,
            Block::Code { code, .. } => code,
            Block::Rule => "---",
        }
    }

    #[test]
    fn paragraphs_lists_code_and_links_become_blocks() {
        let api = ElementId::from_raw(7);
        let resolve = |name: &str| (name == "UrlShortener::api").then_some(api);
        let blocks = parse(
            "# Plan\n\nAdd **two** parts:\n\n- `UrlShortener::api`\n- `other`\n\n1. first\n2. second\n\n```sysml\npart api;\n```\n",
            Theme::default(),
            &resolve,
        );
        let texts: Vec<&str> = blocks.iter().map(text_of).collect();
        assert_eq!(
            texts,
            [
                "Plan",
                "Add two parts:",
                "UrlShortener::api",
                "other",
                "first",
                "second",
                "part api;"
            ]
        );
        let Block::Text(item) = &blocks[2] else {
            panic!("a list item")
        };
        assert_eq!(item.marker.as_deref(), Some("•"));
        assert_eq!(item.links, vec![(0..17, api)]);
        let Block::Text(other) = &blocks[3] else {
            panic!("a list item")
        };
        assert!(other.links.is_empty(), "no element named `other`");
        let Block::Text(second) = &blocks[5] else {
            panic!("a list item")
        };
        assert_eq!(second.marker.as_deref(), Some("2."));
        assert!(matches!(&blocks[6], Block::Code { language, .. } if language == "sysml"));
        // Bold uses the semibold font, not a synthetic weight.
        let Block::Text(paragraph) = &blocks[1] else {
            panic!("a paragraph")
        };
        assert!(
            paragraph
                .job
                .sections
                .iter()
                .any(|s| s.format.font_id == theme::semibold(theme::BODY))
        );
    }
}
