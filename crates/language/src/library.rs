//! The built-in library: the few standard library elements the subset needs,
//! with the pinned libraries' qualified names. Parsed once, shared, immutable.
//! See docs/deviations.md: the pinned `ScalarValues` declares KerML
//! `datatype`s; here they are `attribute def`s, the SysML equivalent.

use crate::parser::{Source, parse_into};
use crate::tree::Tree;
use std::sync::OnceLock;

pub const LIBRARY_TEXT: &str = "\
package ScalarValues {
    abstract attribute def ScalarValue;
    attribute def Boolean :> ScalarValue;
    attribute def String :> ScalarValue;
    abstract attribute def NumericalValue :> ScalarValue;
    abstract attribute def Number :> NumericalValue;
    attribute def Complex :> Number;
    attribute def Real :> Complex;
    attribute def Rational :> Real;
    attribute def Integer :> Rational;
    attribute def Natural :> Integer;
    attribute def Positive :> Natural;
}
";

/// Library ids start here so they never collide with model ids.
const FIRST_LIBRARY_ID: u64 = 1 << 48;

/// The shared library tree.
pub fn library() -> &'static Tree {
    static LIBRARY: OnceLock<Tree> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let mut tree = Tree::with_first_id(FIRST_LIBRARY_ID);
        parse_into(
            &mut tree,
            &[Source::new("<built-in library>", LIBRARY_TEXT)],
        );
        tree
    })
}
