//! The identity and lock file: which element id each element in the text
//! carries, and which elements are locked. History stores both as strings;
//! the System State decides what an id and a locator are.
use crate::{Error, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// File name of the identity and lock file inside the model folder.
pub const IDENTITY_FILE: &str = "agentique.json";
/// The only file format this version reads and writes.
pub const FORMAT: u64 = 1;

/// Element identities and locks for one model folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identities {
    /// Element id to locator: text written by the System State that finds
    /// the element in the documents, such as `part def Shop::Store`.
    pub elements: BTreeMap<String, String>,
    /// Ids of elements the Operator has locked.
    pub locks: BTreeSet<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV1 {
    #[allow(dead_code)]
    format: u64,
    elements: BTreeMap<String, String>,
    #[serde(default)]
    locks: BTreeSet<String>,
}

impl Identities {
    /// Reads the file, refusing any format other than [`FORMAT`].
    pub fn parse(text: &str) -> Result<Self> {
        let invalid = |message: String| Error::Invalid {
            path: IDENTITY_FILE.into(),
            message,
        };
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| invalid(e.to_string()))?;
        let found = value
            .get("format")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid("missing \"format\"".into()))?;
        if found != FORMAT {
            return Err(Error::UnknownFormat {
                path: IDENTITY_FILE.into(),
                found,
            });
        }
        let file: FileV1 = serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
        Ok(Self {
            elements: file.elements,
            locks: file.locks,
        })
    }

    /// Writes one entry per line, sorted by id, so that renaming or moving
    /// an element changes exactly its own line.
    pub fn to_text(&self) -> String {
        let quote = |s: &str| serde_json::to_string(s).expect("strings serialize");
        let mut out = format!("{{\n  \"format\": {FORMAT},\n  \"elements\": ");
        let elements = self
            .elements
            .iter()
            .map(|(id, locator)| format!("{}: {}", quote(id), quote(locator)));
        block(&mut out, '{', '}', elements);
        out.push_str(",\n  \"locks\": ");
        block(&mut out, '[', ']', self.locks.iter().map(|id| quote(id)));
        out.push_str("\n}\n");
        out
    }
}

fn block(out: &mut String, open: char, close: char, lines: impl Iterator<Item = String>) {
    out.push(open);
    let mut empty = true;
    for line in lines {
        out.push_str(if empty { "\n    " } else { ",\n    " });
        out.push_str(&line);
        empty = false;
    }
    if !empty {
        out.push_str("\n  ");
    }
    out.push(close);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_one_entry_per_line() {
        let mut ids = Identities::default();
        ids.elements
            .insert("b2".into(), "part def Shop::Api".into());
        ids.elements
            .insert("a1".into(), "part def Shop::\"Store\"".into());
        ids.locks.insert("a1".into());
        let text = ids.to_text();
        assert_eq!(
            text,
            "{\n  \"format\": 1,\n  \"elements\": {\n    \"a1\": \"part def Shop::\\\"Store\\\"\",\n    \"b2\": \"part def Shop::Api\"\n  },\n  \"locks\": [\n    \"a1\"\n  ]\n}\n"
        );
        assert_eq!(Identities::parse(&text).unwrap(), ids);
        let empty = Identities::default().to_text();
        assert_eq!(
            empty,
            "{\n  \"format\": 1,\n  \"elements\": {},\n  \"locks\": []\n}\n"
        );
        assert_eq!(Identities::parse(&empty).unwrap(), Identities::default());
    }

    #[test]
    fn refuses_unknown_formats_and_fields() {
        let newer = r#"{"format": 2, "elements": {}, "locks": []}"#;
        assert!(matches!(
            Identities::parse(newer),
            Err(Error::UnknownFormat { found: 2, .. })
        ));
        assert!(matches!(
            Identities::parse(r#"{"elements": {}}"#),
            Err(Error::Invalid { .. })
        ));
        assert!(matches!(
            Identities::parse(r#"{"format": 1, "elements": {}, "extra": 1}"#),
            Err(Error::Invalid { .. })
        ));
    }
}
