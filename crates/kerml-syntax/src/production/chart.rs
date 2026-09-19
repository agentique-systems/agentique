//! Finite chart recognition; all input traversal and tree materialization are iterative.
use super::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    rule: usize,
    dot: usize,
    start: usize,
}
#[derive(Default)]
struct Column {
    positions: HashMap<Key, usize>,
    items: Vec<(Key, Vec<usize>)>,
    waiting: HashMap<u16, Vec<usize>>,
}
struct Completed {
    kind: Option<Production>,
    start: usize,
    end: usize,
    children: Vec<usize>,
}
struct Chart {
    columns: Vec<Column>,
    completed: HashMap<(u16, usize, usize), usize>,
    nodes: Vec<Completed>,
    items: usize,
    budget: usize,
}
impl Chart {
    fn insert(&mut self, end: usize, key: Key, children: Vec<usize>) -> Result<(), SourceError> {
        let column = &mut self.columns[end];
        if column.positions.contains_key(&key) {
            return Ok(());
        }
        if self.items >= self.budget {
            return Err(SourceError::Limit("grammar chart"));
        }
        self.items += 1;
        column.positions.insert(key, column.items.len());
        column.items.push((key, children));
        Ok(())
    }
}

pub(super) fn parse(out: &mut Document, budget: usize) -> Result<(), SourceError> {
    let significant: Vec<_> = out
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (!t.kind.is_trivia()).then_some(i))
        .collect();
    let mut rules_by_name = vec![vec![]; generated::SYMBOL_COUNT];
    for (i, rule) in generated::RULES.iter().enumerate() {
        rules_by_name[rule.lhs as usize].push(i);
    }
    let mut chart = Chart {
        columns: (0..=significant.len()).map(|_| Column::default()).collect(),
        completed: HashMap::new(),
        nodes: vec![],
        items: 0,
        budget,
    };
    for &rule in &rules_by_name[generated::ROOT as usize] {
        chart.insert(
            0,
            Key {
                rule,
                dot: 0,
                start: 0,
            },
            vec![],
        )?;
    }
    for end in 0..chart.columns.len() {
        let mut index = 0;
        while index < chart.columns[end].items.len() {
            let (key, children) = chart.columns[end].items[index].clone();
            let item_index = index;
            index += 1;
            let rule = &generated::RULES[key.rule];
            if key.dot == rule.rhs.len() {
                let completed_key = (rule.lhs, key.start, end);
                if chart.completed.contains_key(&completed_key) {
                    continue;
                }
                let node = chart.nodes.len();
                chart.nodes.push(Completed {
                    kind: rule.kind,
                    start: key.start,
                    end,
                    children,
                });
                chart.completed.insert(completed_key, node);
                let parents = chart.columns[key.start]
                    .waiting
                    .get(&rule.lhs)
                    .cloned()
                    .unwrap_or_default();
                for parent in parents {
                    let (mut key, mut children) = chart.columns[key.start].items[parent].clone();
                    key.dot += 1;
                    children.push(node);
                    chart.insert(end, key, children)?;
                }
            } else {
                match rule.rhs[key.dot] {
                    Symbol::Nonterminal(symbol) => {
                        chart.columns[end]
                            .waiting
                            .entry(symbol)
                            .or_default()
                            .push(item_index);
                        for &rule in &rules_by_name[symbol as usize] {
                            chart.insert(
                                end,
                                Key {
                                    rule,
                                    dot: 0,
                                    start: end,
                                },
                                vec![],
                            )?;
                        }
                        if let Some(&empty) = chart.completed.get(&(symbol, end, end)) {
                            let mut children = children;
                            children.push(empty);
                            chart.insert(
                                end,
                                Key {
                                    dot: key.dot + 1,
                                    ..key
                                },
                                children,
                            )?;
                        }
                    }
                    terminal => {
                        if let Some(&i) = significant.get(end) {
                            let token = &out.tokens[i];
                            if matches(terminal, token, out.token_text(token)) {
                                chart.insert(
                                    end + 1,
                                    Key {
                                        dot: key.dot + 1,
                                        ..key
                                    },
                                    children,
                                )?;
                            }
                        }
                    }
                }
            }
        }
    }
    let complete = chart
        .completed
        .get(&(generated::ROOT, 0, significant.len()))
        .copied();
    let root = complete.or_else(|| {
        (0..significant.len())
            .rev()
            .find_map(|end| chart.completed.get(&(generated::ROOT, 0, end)).copied())
    });
    if let Some(root) = root {
        materialize(out, &chart.nodes, root, &significant);
    }
    if complete.is_none() {
        let start = root.map_or(0, |r| chart.nodes[r].end);
        let start = significant
            .get(start)
            .map_or(out.source.len(), |&i| out.tokens[i].range.start() as usize);
        let span = crate::range(start, out.source.len());
        out.recovery.push(span);
        out.diagnostics.push(SyntaxDiagnostic {
            code: "KG_RECOVERY",
            range: span,
            message:
                "Incomplete or unsupported grammar; retained source is not a semantic declaration"
                    .into(),
        });
    }
    Ok(())
}
fn matches(symbol: Symbol, token: &Token, text: &str) -> bool {
    match symbol {
        Symbol::Text(expected) => text == expected,
        Symbol::Name => {
            token.kind == TokenKind::QuotedName
                || (token.kind == TokenKind::Word
                    && generated::KEYWORDS.binary_search(&text).is_err())
        }
        Symbol::String => token.kind == TokenKind::StringValue,
        Symbol::Comment => token.kind == TokenKind::Comment,
        Symbol::Decimal => token.kind == TokenKind::DecimalValue,
        Symbol::Exponential => token.kind == TokenKind::ExponentialValue,
        Symbol::Nonterminal(_) => unreachable!(),
    }
}
fn materialize(out: &mut Document, nodes: &[Completed], root: usize, significant: &[usize]) {
    // Expand a derivation as a tree: repeated empty productions must have distinct
    // syntax identities. Synthetic EBNF/precedence nodes are transparent wrappers.
    let mut pending = vec![(root, None)];
    while let Some((index, parent)) = pending.pop() {
        let node = &nodes[index];
        let parent = if let Some(kind) = node.kind {
            let lo = significant
                .get(node.start)
                .map_or(out.source.len() as u64, |&i| out.tokens[i].range.start());
            let hi = if node.end > node.start {
                out.tokens[significant[node.end - 1]].range.end()
            } else {
                lo
            };
            let index = NodeIndex(out.nodes.len());
            out.nodes.push(NodeData {
                kind,
                id: SyntaxNodeId::from_u128(
                    uuid::Uuid::new_v5(
                        &uuid::Uuid::from_u128(0xe7457369f34c52f88d67bf3e2c68b407),
                        format!(
                            "agq-kerml-production/1:{}:{}:{}",
                            out.revision,
                            index.0,
                            kind.name()
                        )
                        .as_bytes(),
                    )
                    .as_u128(),
                ),
                range: ByteRange::new(lo, hi).expect("chart range"),
                children: vec![],
            });
            if let Some(NodeIndex(parent)) = parent {
                out.nodes[parent].children.push(index);
            } else {
                out.roots.push(index);
            }
            Some(index)
        } else {
            parent
        };
        pending.extend(node.children.iter().rev().map(|&child| (child, parent)));
    }
}
