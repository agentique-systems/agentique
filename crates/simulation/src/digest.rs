//! Provenance digests (ROADMAP §4.14): which part of the model a result
//! depended on, so freshness can be computed instead of stored.
//!
//! The model slice of a scenario is the scenario and everything it reaches
//! through references, with the members of every element reached (a
//! definition's features and behaviour), transitively. Its digest changes
//! when anything in the slice changes, including names, which the harness
//! uses. Elements the slice does not reach do not change it. This is
//! explicit dependency tracking where the model gives it, and conservative
//! elsewhere: any change inside the slice outdates the result.

use agq_language::{Element, ElementId, LIBRARY_TEXT, Tree};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// The elements (of the tree) a scenario depends on.
pub fn closure(tree: &Tree, scenario: ElementId) -> BTreeSet<ElementId> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![scenario];
    while let Some(id) = pending.pop() {
        let Some(element) = tree.get(id) else {
            continue; // the built-in library, or gone
        };
        if !seen.insert(id) {
            continue;
        }
        pending.extend(element.children().iter().copied());
        for (_, reference) in element.references() {
            pending.extend(reference.steps.iter().filter_map(|step| step.target));
        }
    }
    seen
}

/// The digest of a scenario's model slice: a hex SHA-256.
pub fn model_digest(tree: &Tree, scenario: ElementId) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agentique model slice 1\n");
    hasher.update(Sha256::digest(LIBRARY_TEXT.as_bytes()));
    for id in closure(tree, scenario) {
        hasher.update(id.raw().to_le_bytes());
        hasher.update(format!("{:?}", meaning(&tree[id])).as_bytes());
    }
    hex(&hasher.finalize())
}

/// A hex SHA-256 of any text, for recording keys and case versions.
pub fn text_digest(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// An element without its source location and without the written names of
/// linked references, whose targets carry their meaning.
fn meaning(element: &Element) -> Element {
    let mut element = element.clone();
    element.location = None;
    for reference in element.references_mut() {
        for step in &mut reference.steps {
            if step.target.is_some() {
                step.name = agq_language::QualifiedName::default();
            }
        }
    }
    element
}
