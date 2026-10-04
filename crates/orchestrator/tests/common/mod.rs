//! What the evaluations share.

use std::path::{Path, PathBuf};

/// The results file `variable` names, if it is an absolute path outside the
/// repository (evaluation results are never committed); otherwise it says
/// why the results are not written.
pub fn outside_the_repository(variable: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(variable)?);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()?;
    let parent = path
        .parent()
        .filter(|p| path.is_absolute() && !p.as_os_str().is_empty())
        .and_then(|p| p.canonicalize().ok());
    match parent {
        Some(parent) if !parent.starts_with(&repository) => Some(path),
        Some(_) => {
            eprintln!("{variable} is inside the repository: the results are not written");
            None
        }
        None => {
            eprintln!(
                "{variable} must be an absolute path in an existing folder: the results are not written"
            );
            None
        }
    }
}
