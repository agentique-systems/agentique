//! What the Conversation copies and what screen readers hear, without a
//! window: the same functions the view draws with.
use super::*;
use agq_assistant::ToolResult;
use serde_json::json;

fn blocks(text: &str) -> Vec<Block> {
    markdown::parse(text, &|_| None)
}

#[test]
fn a_reply_is_copied_as_markdown_and_its_table_is_shown() {
    let markdown =
        "Two parts:\n\n| Part | Type |\n|---|---|\n| api | HttpApi |\n| store | LinkStore |";
    let Entry::Assistant { parts, .. } = Entry::reply(
        None,
        &[
            json!({ "type": "text", "text": markdown }),
            json!({ "type": "tool_use", "id": "t1", "name": "read_model", "input": {} }),
        ],
    ) else {
        panic!("a reply");
    };
    // Copy puts the reply's Markdown on the clipboard, not its tool calls.
    assert_eq!(reply_markdown(&parts), markdown);
    // The table is drawn as a table.
    let parsed = blocks(markdown);
    assert!(parsed.iter().any(|block| matches!(
        block,
        Block::Table { header, rows } if header == &["Part", "Type"] && rows.len() == 2 && rows[1] == ["store", "LinkStore"]
    )));
}

#[test]
fn a_reply_in_several_texts_is_copied_in_order() {
    let Entry::Assistant { parts, .. } = Entry::reply(
        None,
        &[
            json!({ "type": "text", "text": "First." }),
            json!({ "type": "tool_use", "id": "t1", "name": "read_model", "input": {} }),
            json!({ "type": "text", "text": "  " }),
            json!({ "type": "text", "text": "Second." }),
        ],
    ) else {
        panic!("a reply");
    };
    assert_eq!(reply_markdown(&parts), "First.\n\nSecond.");
}

#[test]
fn messages_and_tool_cards_have_screen_reader_names() {
    assert_eq!(
        message_name("You", &blocks("Add an API")),
        "You: Add an API"
    );
    assert_eq!(
        message_name("Assistant", &blocks("Added `P::api`.")),
        "Assistant: Added P::api."
    );
    let tool = |result: Option<ToolResult>, running: bool| cards::Tool {
        id: "t1".into(),
        name: agq_assistant::tools::APPLY_CHANGES.into(),
        input: json!({ "description": "Add the API", "operations": [] }),
        result,
        running,
        outcome: None,
        options: None,
    };
    let done = ToolResult {
        tool_use_id: "t1".into(),
        content: "Changed.".into(),
        is_error: false,
        change: None,
    };
    assert_eq!(
        tool(Some(done), false).accessible_name(),
        "Tool call: Add the API, done"
    );
    assert_eq!(
        tool(None, true).accessible_name(),
        "Tool call: Add the API, running"
    );
}
