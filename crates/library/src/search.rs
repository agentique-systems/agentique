//! Searching the index: a fuzzy matcher that also returns what it matched,
//! so results can mark the characters, and ranked search with filters.

use crate::{Index, Scope};
use agq_language::ElementKind;

/// A fuzzy match: higher scores are better; `positions` are the matched
/// characters (char indices into the text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    pub positions: Vec<usize>,
}

/// Matches `query` against `text` without regard to case: every
/// non-space character of the query, in order. Matches at word starts
/// (the text's start, after `:`, `.`, `_`, `-`, `/` or a space, and at a
/// lower-to-upper case change), consecutive runs and early matches score
/// higher; the best-scoring alignment is chosen, and an exact match of the
/// whole text scores highest.
pub fn fuzzy(query: &str, text: &str) -> Option<Match> {
    let needle: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if needle.is_empty() {
        return Some(Match {
            score: 0,
            positions: Vec::new(),
        });
    }
    let original: Vec<char> = text.chars().collect();
    let lower: Vec<char> = original
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let (n, m) = (needle.len(), lower.len());
    if n > m {
        return None;
    }
    let bonus = |j: usize| -> i32 {
        if j == 0 {
            return 10;
        }
        let (before, here) = (original[j - 1], original[j]);
        if matches!(before, ':' | '.' | '_' | '-' | '/' | ' ') {
            9
        } else if before.is_lowercase() && here.is_uppercase() {
            8
        } else if before.is_alphabetic() && here.is_numeric() {
            4
        } else {
            0
        }
    };
    // score[i * m + j]: the best alignment of needle[..=i] with needle[i]
    // at text position j; from[..]: where needle[i - 1] went.
    const NONE: i32 = i32::MIN / 4;
    let mut score = vec![NONE; n * m];
    let mut from = vec![usize::MAX; n * m];
    for i in 0..n {
        // The best `score[i - 1][k] + k` over k < j - 1 (a gap follows k).
        let (mut best_gap, mut best_k) = (NONE, usize::MAX);
        for j in 0..m {
            if i > 0 && j >= 2 {
                let k = j - 2;
                let previous = score[(i - 1) * m + k];
                if previous > NONE && previous + k as i32 > best_gap {
                    best_gap = previous + k as i32;
                    best_k = k;
                }
            }
            if lower[j] != needle[i] {
                continue;
            }
            let here = 16 + bonus(j);
            if i == 0 {
                score[j] = here - j as i32 / 2;
                continue;
            }
            let (mut value, mut k_best) = (NONE, usize::MAX);
            if j >= 1 && score[(i - 1) * m + j - 1] > NONE {
                value = score[(i - 1) * m + j - 1] + 8;
                k_best = j - 1;
            }
            // Skipping `j - k - 1` characters costs one more than that.
            if best_gap > NONE && best_gap - j as i32 - 1 > value {
                value = best_gap - j as i32 - 1;
                k_best = best_k;
            }
            if value > NONE {
                score[i * m + j] = here + value;
                from[i * m + j] = k_best;
            }
        }
    }
    let (end, best) = (0..m)
        .map(|j| (j, score[(n - 1) * m + j]))
        .filter(|(_, s)| *s > NONE)
        .max_by_key(|(j, s)| (*s, -(*j as i32)))?;
    let mut positions = vec![end];
    let mut j = end;
    for i in (1..n).rev() {
        j = from[i * m + j];
        positions.push(j);
    }
    positions.reverse();
    let exact = if lower == needle { 40 } else { 0 };
    Some(Match {
        score: best + exact,
        positions,
    })
}

/// What to search for.
#[derive(Clone, Debug, Default)]
pub struct Query<'a> {
    /// Words, each matched fuzzily against a block's name, qualified name,
    /// documentation, category and feature names; all must match.
    pub text: &'a str,
    /// Only this scope; all when `None`.
    pub scope: Option<Scope>,
    /// Only these kinds; all when empty.
    pub kinds: &'a [ElementKind],
    /// Only these blocks (indexes into the index), such as the blocks that
    /// fit a port; all when `None`.
    pub only: Option<&'a [usize]>,
    /// At most this many hits; all when 0.
    pub limit: usize,
}

/// One result: the block (an index into [`Index::blocks`]), its score and
/// the matched characters of its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub block: usize,
    pub score: i32,
    pub name_positions: Vec<usize>,
}

impl Index {
    /// Blocks matching `query`, best first. Without text, every block that
    /// passes the filters, by scope, category and name. A built-in or My
    /// Library block the project already holds unchanged is found as the
    /// project's copy, unless only its own scope is searched.
    pub fn search(&self, query: &Query) -> Vec<Hit> {
        let words: Vec<&str> = query.text.split_whitespace().collect();
        let mut hits: Vec<Hit> = Vec::new();
        for (i, block) in self.blocks().iter().enumerate() {
            if query.scope.is_some_and(|s| s != block.reference.scope)
                || (!query.kinds.is_empty() && !query.kinds.contains(&block.kind))
                || query.only.is_some_and(|only| !only.contains(&i))
            {
                continue;
            }
            if query.scope.is_none() && self.project_copy(block).is_some() {
                continue;
            }
            let mut score = 0;
            let mut name_positions = Vec::new();
            let mut matched = true;
            for word in &words {
                // The name first; the qualified name and the words only
                // when the name does not match.
                let name = subsequence(word, &block.keys.name)
                    .then(|| fuzzy(word, &block.name))
                    .flatten();
                let best = match &name {
                    Some(m) => Some(m.score * 3),
                    None => subsequence(word, &block.keys.qualified)
                        .then(|| fuzzy(word, &block.keys.qualified).map(|m| m.score))
                        .flatten()
                        .or_else(|| contains_word(&block.keys.words, word).then_some(14)),
                };
                match best {
                    Some(best) => {
                        score += best;
                        if let Some(name) = name {
                            name_positions.extend(name.positions);
                        }
                    }
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
            if !matched {
                continue;
            }
            if !words.is_empty() && block.keys.name == query.text.trim().to_lowercase() {
                score += 1000;
            }
            // Concrete blocks with a purpose come before abstract ones and
            // the standard library's value types.
            if block.is_abstract || block.standard {
                score -= 6;
            }
            name_positions.sort_unstable();
            name_positions.dedup();
            hits.push(Hit {
                block: i,
                score,
                name_positions,
            });
        }
        let blocks = self.blocks();
        if words.is_empty() {
            // Browsing: composites first, then the other components, then
            // the vocabulary they are built from (ports, items), then
            // requirements and value types.
            let rank = |b: &crate::Block| match b.kind {
                ElementKind::PartDef if b.composite() => 0,
                ElementKind::PartDef => 1,
                ElementKind::PortDef | ElementKind::InterfaceDef | ElementKind::ConnectionDef => 2,
                ElementKind::ItemDef => 3,
                ElementKind::RequirementDef => 4,
                _ => 5,
            };
            hits.sort_by(|a, b| {
                let (a, b) = (&blocks[a.block], &blocks[b.block]);
                (a.reference.scope, a.standard, rank(a), &a.category, &a.name).cmp(&(
                    b.reference.scope,
                    b.standard,
                    rank(b),
                    &b.category,
                    &b.name,
                ))
            });
        } else {
            hits.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then_with(|| blocks[a.block].name.len().cmp(&blocks[b.block].name.len()))
                    .then_with(|| blocks[a.block].name.cmp(&blocks[b.block].name))
            });
        }
        if query.limit > 0 {
            hits.truncate(query.limit);
        }
        hits
    }
}

/// Whether the characters of `word` appear in order in `text` (lower
/// case), without regard to case: a quick test before scoring.
fn subsequence(word: &str, text: &str) -> bool {
    let mut rest = text.chars();
    word.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .all(|c| rest.any(|t| t == c))
}

/// Whether a word of `words` (lower case) starts with `word`, so that
/// "limit" finds "limitPerMinute" in the documentation without stray
/// subsequence matches across the whole text.
fn contains_word(words: &str, word: &str) -> bool {
    let word = word.to_lowercase();
    words
        .split(|c: char| !c.is_alphanumeric())
        .any(|candidate| candidate.starts_with(&word))
}

#[cfg(test)]
mod tests {
    use super::fuzzy;

    #[test]
    fn word_starts_and_runs_win() {
        let cached = fuzzy("cs", "CachedStore").unwrap();
        assert_eq!(cached.positions, [0, 6]);
        let kv = fuzzy("kvs", "KeyValueStore").unwrap();
        assert_eq!(kv.positions, [0, 3, 8]);
        assert!(
            fuzzy("store", "Store").unwrap().score > fuzzy("store", "KeyValueStore").unwrap().score
        );
        assert!(
            fuzzy("cache", "Cache").unwrap().score > fuzzy("cache", "CachedStore").unwrap().score
        );
        assert!(fuzzy("xyz", "CachedStore").is_none());
        assert_eq!(
            fuzzy("", "Anything").unwrap().positions,
            Vec::<usize>::new()
        );
    }

    #[test]
    fn qualified_names_match_by_segment() {
        let m = fuzzy("storage::cache", "Library::Storage::Cache").unwrap();
        assert_eq!(m.positions.len(), "storage::cache".len());
        let m = fuzzy("rl", "Library::Services::RateLimiter").unwrap();
        assert_eq!(m.positions, [19, 23]);
    }
}
