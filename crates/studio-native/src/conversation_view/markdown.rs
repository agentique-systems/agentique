//! Markdown in the Conversation: `pulldown-cmark` turns a message into
//! blocks (paragraphs, headings, lists, quotes, fenced code, tables, rules)
//! of text with styled spans. An inline code span that names an element of
//! the model is a link. The blocks are drawn as GPUI text runs; text is
//! selected by place in the text ([`TextPoint`]), so a selection survives
//! scrolling, virtualisation and replies streaming in below it.
use agq_language::ElementId;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// One block of a message.
#[derive(Clone, Debug)]
pub enum Block {
    Text(TextBlock),
    Code {
        language: String,
        code: String,
        indent: usize,
        /// A list item's bullet or number, when the item starts with code.
        marker: Option<String>,
    },
    Rule,
    /// A table: its header and rows of plain cell text.
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

impl Block {
    /// The block's text as it is copied: a table's cells are separated by
    /// tabs and its rows by lines, so it pastes into a spreadsheet.
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Block::Text(block) => block.text.as_str().into(),
            Block::Code { code, .. } => code.as_str().into(),
            Block::Rule => "---".into(),
            Block::Table { header, rows } => std::iter::once(header)
                .chain(rows.iter())
                .map(|row| row.join("\t"))
                .collect::<Vec<_>>()
                .join("\n")
                .into(),
        }
    }

    fn marker(&self) -> Option<&str> {
        match self {
            Block::Text(block) => block.marker.as_deref(),
            Block::Code { marker, .. } => marker.as_deref(),
            _ => None,
        }
    }
}

/// How a span of text looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// An element link (a code span naming an element).
    pub link: bool,
}

/// A paragraph, heading or list item.
#[derive(Clone, Debug, Default)]
pub struct TextBlock {
    pub text: String,
    /// Styled byte ranges of `text`, in order, covering it.
    pub spans: Vec<(Range<usize>, Style)>,
    /// Element links, as byte ranges of `text`.
    pub links: Vec<(Range<usize>, ElementId)>,
    /// Nesting of lists and quotes.
    pub indent: usize,
    /// A list item's bullet or number.
    pub marker: Option<String>,
    pub quote: bool,
    /// 1 or 2 for headings.
    pub heading: Option<u8>,
}

/// A place in the Conversation's text: a message (its entry and slot, in the
/// order shown), a block of it, and a byte of that block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPoint {
    pub message: (usize, usize),
    pub block: usize,
    pub offset: usize,
}

pub type BlockKey = ((usize, usize), usize);

impl TextPoint {
    pub fn block_key(self) -> BlockKey {
        (self.message, self.block)
    }
}

/// Text selected across blocks and messages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPoint,
    pub focus: TextPoint,
    /// The pointer that started it is still down.
    pub dragging: bool,
}

impl TextSelection {
    /// Its ends, first to last.
    pub fn range(&self) -> (TextPoint, TextPoint) {
        (self.anchor.min(self.focus), self.anchor.max(self.focus))
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.focus
    }
}

/// The selected bytes of block `key`, which has `length` bytes.
pub fn block_range((from, to): (TextPoint, TextPoint), key: BlockKey, length: usize) -> Option<Range<usize>> {
    if key < from.block_key() || key > to.block_key() {
        return None;
    }
    let start = if key == from.block_key() { from.offset.min(length) } else { 0 };
    let end = if key == to.block_key() { to.offset.min(length) } else { length };
    (start < end).then_some(start..end)
}

/// A message's whole text, as [`selected_text`] copies it.
pub fn plain_text(blocks: &[Block]) -> String {
    let all = (
        TextPoint::default(),
        TextPoint {
            message: (0, 0),
            block: usize::MAX,
            offset: usize::MAX,
        },
    );
    selected_text([((0, 0), blocks)], all)
}

/// The text of a selection, in order: blocks of a message are separated by
/// a blank line (list items by a line break), messages by a blank line. A
/// list item selected from its start keeps its bullet or number.
pub fn selected_text<'a>(
    messages: impl IntoIterator<Item = ((usize, usize), &'a [Block])>,
    range: (TextPoint, TextPoint),
) -> String {
    let mut text = String::new();
    let mut previous: Option<((usize, usize), bool)> = None;
    for (message, blocks) in messages {
        for (index, block) in blocks.iter().enumerate() {
            let content = block.text();
            let Some(selected) = block_range(range, (message, index), content.len()) else {
                continue;
            };
            // Keep to character boundaries.
            let start = floor_boundary(&content, selected.start);
            let end = floor_boundary(&content, selected.end);
            let item = block.marker().is_some();
            if let Some((last, last_item)) = previous {
                text.push_str(if last == message && item && last_item { "\n" } else { "\n\n" });
            }
            if let Some(marker) = block.marker().filter(|_| start == 0) {
                text.push_str(marker);
                text.push(' ');
            }
            text.push_str(&content[start..end]);
            previous = Some((message, item));
        }
    }
    text
}

pub fn floor_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Parses `text` into blocks. `resolve` names the element an inline code
/// span refers to, if any.
pub fn parse(text: &str, resolve: &dyn Fn(&str) -> Option<ElementId>) -> Vec<Block> {
    let mut builder = Builder {
        resolve,
        blocks: Vec::new(),
        current: None,
        marker: None,
        style: Style::default(),
        bold: 0,
        italic: 0,
        strike: 0,
        heading: None,
        lists: Vec::new(),
        quote: 0,
        code: None,
        table: None,
    };
    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES) {
        builder.event(event);
    }
    builder.flush();
    builder.blocks
}

#[derive(Default)]
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
}

struct Builder<'a> {
    resolve: &'a dyn Fn(&str) -> Option<ElementId>,
    blocks: Vec<Block>,
    current: Option<TextBlock>,
    marker: Option<String>,
    style: Style,
    bold: usize,
    italic: usize,
    strike: usize,
    heading: Option<HeadingLevel>,
    /// Open lists: the next number of an ordered list.
    lists: Vec<Option<u64>>,
    quote: usize,
    code: Option<Block>,
    table: Option<Table>,
}

impl Builder<'_> {
    fn event(&mut self, event: Event) {
        if let Some(Block::Code { code, .. }) = &mut self.code {
            match event {
                Event::Text(text) => code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let mut block = self.code.take().expect("inside a code block");
                    if let Block::Code { code, .. } = &mut block
                        && code.ends_with('\n')
                    {
                        code.pop();
                    }
                    self.blocks.push(block);
                }
                _ => {}
            }
            return;
        }
        if let Some(table) = &mut self.table {
            match event {
                Event::Text(text) => table.cell.push_str(&text),
                Event::Code(code) => table.cell.push_str(&code),
                Event::SoftBreak | Event::HardBreak => table.cell.push(' '),
                Event::Start(Tag::TableRow) => table.row.clear(),
                Event::End(TagEnd::TableCell) => {
                    let cell = std::mem::take(&mut table.cell);
                    table.row.push(cell.trim().to_string());
                }
                Event::End(TagEnd::TableHead) => table.header = std::mem::take(&mut table.row),
                Event::End(TagEnd::TableRow) => {
                    let row = std::mem::take(&mut table.row);
                    table.rows.push(row);
                }
                Event::End(TagEnd::Table) => {
                    let table = self.table.take().expect("inside a table");
                    self.blocks.push(Block::Table {
                        header: table.header,
                        rows: table.rows,
                    });
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Table(_) => {
                    self.flush();
                    self.table = Some(Table::default());
                }
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
                    self.code = Some(Block::Code {
                        language,
                        code: String::new(),
                        indent: self.indent(),
                        marker: self.marker.take(),
                    });
                }
                Tag::BlockQuote(_) => {
                    self.flush();
                    self.quote += 1;
                }
                Tag::Strong => self.bold += 1,
                Tag::Emphasis => self.italic += 1,
                Tag::Strikethrough => self.strike += 1,
                // Web links are shown as their text: the Studio opens no
                // browser, so nothing may look clickable that is not.
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Item | TagEnd::HtmlBlock => self.flush(),
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
            && !block.text.trim().is_empty()
        {
            self.blocks.push(Block::Text(block));
        }
    }

    fn indent(&self) -> usize {
        self.lists.len() + self.quote
    }

    fn block(&mut self) -> &mut TextBlock {
        let indent = self.indent();
        let heading = match self.heading {
            Some(HeadingLevel::H1) => Some(1),
            Some(_) => Some(2),
            None => None,
        };
        let (marker, quote) = (&mut self.marker, self.quote > 0);
        self.current.get_or_insert_with(|| TextBlock {
            indent,
            marker: marker.take(),
            quote,
            heading,
            ..Default::default()
        })
    }

    fn style(&self) -> Style {
        Style {
            bold: self.bold > 0 || self.heading.is_some(),
            italic: self.italic > 0,
            strike: self.strike > 0,
            ..self.style
        }
    }

    fn push(&mut self, text: &str, style: Style) -> Range<usize> {
        let block = self.block();
        let start = block.text.len();
        block.text.push_str(text);
        let end = block.text.len();
        match block.spans.last_mut() {
            Some((range, last)) if *last == style && range.end == start => range.end = end,
            _ => block.spans.push((start..end, style)),
        }
        start..end
    }

    fn text(&mut self, text: &str) {
        let style = self.style();
        self.push(text, style);
    }

    fn code_span(&mut self, code: &str) {
        let element = (self.resolve)(code.trim());
        let style = Style {
            code: true,
            link: element.is_some(),
            ..self.style()
        };
        let range = self.push(code, style);
        if let Some(id) = element {
            self.block().links.push((range, id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(text: &str) -> Vec<Block> {
        parse(text, &|name| (name == "P::api").then(|| agq_language::ElementId::from_raw(7)))
    }

    #[test]
    fn a_reply_parses_into_blocks_with_links_and_styles() {
        let parsed = blocks("# Plan\n\nAdd **the API** as `P::api` and `other`.\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert_eq!(parsed.len(), 6, "{parsed:#?}");
        let Block::Text(heading) = &parsed[0] else { panic!() };
        assert_eq!(heading.heading, Some(1));
        let Block::Text(paragraph) = &parsed[1] else { panic!() };
        assert_eq!(paragraph.links.len(), 1);
        let (range, id) = &paragraph.links[0];
        assert_eq!(&paragraph.text[range.clone()], "P::api");
        assert_eq!(id.raw(), 7);
        assert!(paragraph.spans.iter().any(|(range, style)| style.bold && &paragraph.text[range.clone()] == "the API"));
        let Block::Text(item) = &parsed[2] else { panic!() };
        assert_eq!(item.marker.as_deref(), Some("•"));
        assert!(matches!(&parsed[4], Block::Code { language, code, .. } if language == "rust" && code == "fn main() {}"));
        assert!(matches!(&parsed[5], Block::Table { header, rows } if header == &["a", "b"] && rows == &[vec!["1".to_string(), "2".to_string()]]));
    }

    #[test]
    fn a_selection_across_messages_is_copied_in_order_with_list_markers() {
        let first = blocks("First message from you.");
        let second = blocks("The second message.\n\n- one\n- two");
        let third = blocks("Third message here.");
        let all = [((0, 0), first.as_slice()), ((1, 0), second.as_slice()), ((2, 0), third.as_slice())];
        let range = (
            TextPoint { message: (0, 0), block: 0, offset: 0 },
            TextPoint { message: (2, 0), block: 0, offset: 19 },
        );
        assert_eq!(
            selected_text(all, range),
            "First message from you.\n\nThe second message.\n\n• one\n• two\n\nThird message here."
        );
        let table = blocks("| a | b |\n|---|---|\n| 1 | 2 |");
        assert_eq!(plain_text(&table), "a\tb\n1\t2");
    }

    #[test]
    fn a_selection_ending_inside_a_character_keeps_to_its_boundary() {
        let text = blocks("Größe");
        let range = (
            TextPoint::default(),
            TextPoint { message: (0, 0), block: 0, offset: 3 },
        );
        assert_eq!(selected_text([((0, 0), text.as_slice())], range), "Gr");
    }
}
