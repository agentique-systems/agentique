//! Cursor state binds every semantic page to its exact immutable revision.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Maximum records returned by any page.
pub const MAX_PAGE_SIZE: usize = 1_000;

/// Identity of a query whose ordered results can be paginated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageBinding {
    /// Owning project.
    pub project: Uuid,
    /// Resolved immutable project revision, never a mutable branch name.
    pub revision: Uuid,
    /// Exact operation and filter identity (including declared/effective mode).
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    format: u8,
    binding: PageBinding,
    offset: usize,
    size: usize,
}

/// Invalid or cross-revision continuation request.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum PagingError {
    /// The cursor is malformed or its transport checksum differs.
    #[error("malformed continuation cursor")]
    Malformed,
    /// Cursor and request name different immutable results.
    #[error("continuation cursor belongs to another project, revision, or query")]
    BindingMismatch,
    /// Page size is outside the bounded supported range.
    #[error("page size must be between 1 and 1000")]
    Size,
    /// Both direction parameters were supplied.
    #[error("page[before] and page[after] are mutually exclusive")]
    Direction,
}

/// A borrowed page with continuation cursors; wire bodies remain JSON arrays.
#[derive(Clone, Debug)]
pub struct Page<'a, T> {
    /// Values on this page.
    pub values: &'a [T],
    /// Cursor to pass as `page[after]` for the next page.
    pub next: Option<String>,
    /// Cursor to pass as `page[before]` for the previous page.
    pub previous: Option<String>,
}

fn encode(cursor: &Cursor) -> String {
    let bytes = serde_json::to_vec(cursor).expect("cursor serialization cannot fail");
    let mut output = String::with_capacity(bytes.len() * 2 + 65);
    for byte in &bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output.push('.');
    output.push_str(&format!("{:x}", Sha256::digest(bytes)));
    output
}

fn decode(text: &str) -> Result<Cursor, PagingError> {
    // Bound parser allocations before accepting externally supplied cursors.
    if text.len() > 16_384 {
        return Err(PagingError::Malformed);
    }
    let (payload, checksum) = text.split_once('.').ok_or(PagingError::Malformed)?;
    if payload.len() % 2 != 0 || !payload.is_ascii() {
        return Err(PagingError::Malformed);
    }
    let bytes = (0..payload.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&payload[offset..offset + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PagingError::Malformed)?;
    if format!("{:x}", Sha256::digest(&bytes)) != checksum {
        return Err(PagingError::Malformed);
    }
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| PagingError::Malformed)?;
    if cursor.format != 1 || !(1..=MAX_PAGE_SIZE).contains(&cursor.size) {
        return Err(PagingError::Malformed);
    }
    Ok(cursor)
}

/// Resolve the revision carried by a continuation before consulting a branch.
///
/// The checksum detects corruption and is not an authorization credential. Every
/// request must independently authorize access to the returned project/revision.
pub fn continuation_binding(cursor: &str) -> Result<PageBinding, PagingError> {
    Ok(decode(cursor)?.binding)
}

/// Paginate a stable, deterministically ordered immutable result sequence.
pub fn page<'a, T>(
    values: &'a [T],
    binding: &PageBinding,
    size: Option<usize>,
    after: Option<&str>,
    before: Option<&str>,
) -> Result<Page<'a, T>, PagingError> {
    if after.is_some() && before.is_some() {
        return Err(PagingError::Direction);
    }
    let (offset, size) = match after.or(before) {
        Some(text) => {
            let cursor = decode(text)?;
            if cursor.binding != *binding {
                return Err(PagingError::BindingMismatch);
            }
            if size.is_some_and(|size| size != cursor.size) {
                return Err(PagingError::Size);
            }
            let offset = if before.is_some() {
                cursor.offset.saturating_sub(cursor.size)
            } else {
                cursor.offset.saturating_add(cursor.size)
            };
            (offset.min(values.len()), cursor.size)
        }
        None => (0, size.unwrap_or(100)),
    };
    if !(1..=MAX_PAGE_SIZE).contains(&size) {
        return Err(PagingError::Size);
    }
    let end = offset.saturating_add(size).min(values.len());
    let cursor = || {
        encode(&Cursor {
            format: 1,
            binding: binding.clone(),
            offset,
            size,
        })
    };
    Ok(Page {
        values: &values[offset..end],
        next: (end < values.len()).then(cursor),
        previous: (offset != 0).then(cursor),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_keeps_revision_and_order_even_if_branch_moves() {
        let binding = PageBinding {
            project: Uuid::from_u128(1),
            revision: Uuid::from_u128(2),
            query: "getElementsByProjectCommit:effective".into(),
        };
        let first = page(&[1, 2, 3, 4, 5], &binding, Some(2), None, None).unwrap();
        assert_eq!(first.values, &[1, 2]);
        let cursor = first.next.unwrap();
        assert_eq!(continuation_binding(&cursor).unwrap(), binding);
        let second = page(&[1, 2, 3, 4, 5], &binding, None, Some(&cursor), None).unwrap();
        assert_eq!(second.values, &[3, 4]);
        let previous = second.previous.unwrap();
        let first_again = page(&[1, 2, 3, 4, 5], &binding, None, None, Some(&previous)).unwrap();
        assert_eq!(first_again.values, &[1, 2]);
        let moved = PageBinding {
            revision: Uuid::from_u128(3),
            ..binding
        };
        assert_eq!(
            page(&[6, 7, 8], &moved, None, Some(&cursor), None).unwrap_err(),
            PagingError::BindingMismatch
        );
    }

    #[test]
    fn malformed_and_cross_query_cursors_are_rejected() {
        let binding = PageBinding {
            project: Uuid::nil(),
            revision: Uuid::nil(),
            query: "elements".into(),
        };
        assert_eq!(continuation_binding("corrupt"), Err(PagingError::Malformed));
        assert_eq!(
            page(&[1], &binding, Some(0), None, None).unwrap_err(),
            PagingError::Size
        );
        let cursor = page(&[1, 2], &binding, Some(1), None, None)
            .unwrap()
            .next
            .unwrap();
        let other = PageBinding {
            query: "roots".into(),
            ..binding
        };
        assert_eq!(
            page(&[1, 2], &other, None, Some(&cursor), None).unwrap_err(),
            PagingError::BindingMismatch
        );
    }
}
