//! Inputs shared by held Working-state tests and the real parser preflight.
#![allow(dead_code)]

pub const IDENTITY_DOCUMENT: &str =
    "package IdentityStable { part def Retained; }\npackage IdentityEdited { part def Scratch; }\n";
pub const RETAINED_DECLARATION: &str = "part def Retained;";
pub const COMPLETE_EDITED_DECLARATION: &str = "part def Scratch;";
pub const RECOVERED_EDITED_DECLARATION: &str = "part def Scratch {";

// Operational v2 parses this flag; effective variation semantics remain an
// explicit pending capability. A complete producer run cannot erase that status.
pub const VARIATION_DOCUMENT: &str =
    "package Choices { part def Configurable { variation part selected; } }\n";
pub const ORDINARY_DOCUMENT: &str =
    "package Choices { part def Configurable { part selected; } }\n";

pub fn recovered_identity_document() -> String {
    IDENTITY_DOCUMENT.replace(COMPLETE_EDITED_DECLARATION, RECOVERED_EDITED_DECLARATION)
}
