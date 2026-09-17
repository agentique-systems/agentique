use crate::*;

#[derive(Clone, Debug)]
pub struct TextEdit {
    pub range: ByteRange,
    pub replacement: String,
}
impl SyntaxDocument {
    /// Apply one edit against this exact immutable source revision (ADR 0006).
    pub fn edit(&self, edit: &TextEdit, limits: ParseLimits) -> Result<Self, SourceError> {
        if self.text(edit.range).is_none() {
            return Err(SourceError::InvalidEdit);
        }
        let length = self.source.len() - (edit.range.end() - edit.range.start()) as usize;
        if edit.replacement.len() > limits.max_bytes.saturating_sub(length)
            || length > limits.max_bytes
        {
            return Err(SourceError::Limit("byte"));
        }
        let mut source = self.source.to_string();
        source.replace_range(
            edit.range.start() as usize..edit.range.end() as usize,
            &edit.replacement,
        );
        let mut next = parse(self.document, source, limits)?;
        next.root_id = self.root_id;
        let formatting = signature(self, range(0, self.source.len()), None)
            == signature(&next, range(0, next.source.len()), None);
        // Move nodes out temporarily to borrow the next source/token store during matching.
        let mut nodes = std::mem::take(&mut next.nodes);
        reconcile(self, &next, &self.nodes, &mut nodes, edit, formatting);
        next.nodes = nodes;
        Ok(next)
    }
}
fn signature(doc: &SyntaxDocument, span: ByteRange, omit: Option<ByteRange>) -> Vec<&str> {
    let start = doc
        .tokens
        .partition_point(|t| t.range.start() < span.start());
    let end = doc.tokens.partition_point(|t| t.range.end() <= span.end());
    doc.tokens[start..end]
        .iter()
        .filter(|t| !t.kind.is_trivia() && Some(t.range) != omit)
        .map(|t| doc.token_text(t))
        .collect()
}
fn mapped(span: ByteRange, edit: &TextEdit) -> Option<ByteRange> {
    if span.end() <= edit.range.start() {
        return Some(span);
    }
    if span.start() >= edit.range.end() {
        let delta = edit.replacement.len() as i64 - (edit.range.end() - edit.range.start()) as i64;
        return ByteRange::new(
            (span.start() as i64 + delta) as u64,
            (span.end() as i64 + delta) as u64,
        )
        .ok();
    }
    None
}
fn same_reference(a: &ReferenceSyntax, b: &ReferenceSyntax) -> bool {
    a.kind == b.kind
        && a.absolute == b.absolute
        && a.segments
            .iter()
            .map(|n| &n.value)
            .eq(b.segments.iter().map(|n| &n.value))
}
fn reconcile(
    old_doc: &SyntaxDocument,
    new_doc: &SyntaxDocument,
    old: &[SyntaxNode],
    new: &mut [SyntaxNode],
    edit: &TextEdit,
    formatting: bool,
) {
    use std::collections::BTreeMap;
    let declarations: Vec<_> = old
        .iter()
        .filter_map(Declaration::cast)
        .map(Declaration::syntax)
        .collect();
    let mut old_signatures = BTreeMap::<_, Vec<_>>::new();
    let mut new_signatures = BTreeMap::<_, usize>::new();
    let mut anchors = BTreeMap::new();
    if formatting {
        for previous in &declarations {
            old_signatures
                .entry(signature(old_doc, previous.range, None))
                .or_default()
                .push(*previous);
        }
        for next in new.iter().filter_map(Declaration::cast) {
            *new_signatures
                .entry(signature(new_doc, next.syntax().range, None))
                .or_default() += 1;
        }
    } else {
        for previous in declarations {
            if let Some(span) = mapped(previous.keyword, edit) {
                anchors.insert((span.start(), span.end()), previous);
            }
        }
    }
    for node in new {
        let SyntaxNode::Declaration(next) = node else {
            continue;
        };
        let previous = if formatting {
            let sig = signature(new_doc, next.range, None);
            old_signatures
                .get(&sig)
                .filter(|matches| matches.len() == 1 && new_signatures[&sig] == 1)
                .map(|matches| matches[0])
        } else {
            anchors
                .get(&(next.keyword.start(), next.keyword.end()))
                .copied()
                .filter(|previous| {
                    let rename = previous
                        .name
                        .as_ref()
                        .is_some_and(|n| n.range == edit.range);
                    previous.kind == next.kind
                        && signature(
                            old_doc,
                            previous.header,
                            rename.then(|| previous.name.as_ref().unwrap().range),
                        ) == signature(
                            new_doc,
                            next.header,
                            if rename {
                                next.name.as_ref().map(|n| n.range)
                            } else {
                                None
                            },
                        )
                })
        };
        if let Some(previous) = previous {
            next.id = previous.id;
            for reference in &mut next.references {
                let matches: Vec<_> = previous
                    .references
                    .iter()
                    .filter(|r| same_reference(r, reference))
                    .collect();
                if let [found] = matches.as_slice() {
                    reference.id = found.id;
                }
            }
            // Duplicate reference assertions cannot share a reconciled identity.
            let mut seen = std::collections::BTreeSet::new();
            let duplicates: std::collections::BTreeSet<_> = next
                .references
                .iter()
                .filter_map(|r| (!seen.insert(r.id)).then_some(r.id))
                .collect();
            for reference in &mut next.references {
                if duplicates.contains(&reference.id) {
                    reference.id = SyntaxNodeId::new();
                }
            }
            reconcile(
                old_doc,
                new_doc,
                &previous.children,
                &mut next.children,
                edit,
                formatting,
            );
        }
    }
}
