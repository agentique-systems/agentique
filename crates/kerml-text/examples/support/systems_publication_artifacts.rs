//! Candidate files become visible together through one same-volume directory rename.
use agq_kerml_text::sysml::CanonicalSysmlSystemsLibrary;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ArtifactTransaction {
    pub candidate: PathBuf,
    pub destination: PathBuf,
    pub audits: PathBuf,
    started: Instant,
}
impl ArtifactTransaction {
    pub fn begin(report: &Path) -> Result<Self> {
        let destination = report.parent().ok_or("report directory")?.to_path_buf();
        let parent = destination.parent().ok_or("publication parent directory")?;
        let name = destination
            .file_name()
            .ok_or("publication directory name")?
            .to_string_lossy();
        if destination.exists() {
            return Err(
                "accepted artifact directory already exists; select a fresh --output directory"
                    .into(),
            );
        }
        fs::create_dir_all(parent)?;
        let run = uuid::Uuid::new_v4();
        let candidate = parent.join(format!(".{name}.candidate-{run}"));
        let audits = parent.join(format!(".{name}.audits-{run}"));
        fs::create_dir(&candidate)?;
        fs::create_dir(&audits)?;
        Ok(Self {
            candidate,
            destination,
            audits,
            started: Instant::now(),
        })
    }
    pub fn report_path(&self, requested: &Path) -> Result<PathBuf> {
        Ok(self
            .candidate
            .join(requested.file_name().ok_or("report file name")?))
    }
    pub fn stage<T>(&self, name: &str, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        self.event(name, "begin", None, None)?;
        let started = Instant::now();
        let result = operation();
        self.event(
            name,
            "end",
            Some(started.elapsed().as_secs_f64()),
            result.as_ref().err().map(ToString::to_string),
        )?;
        result
    }
    fn event(
        &self,
        name: &str,
        event: &str,
        seconds: Option<f64>,
        error: Option<String>,
    ) -> Result<()> {
        let value = json!({"stage":name,"event":event,"elapsed_seconds":self.started.elapsed().as_secs_f64(),
            "stage_seconds":seconds,"error":error,"grants_publication_authority":false});
        let mut events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.audits.join("artifact-stages.jsonl"))?;
        writeln!(events, "{value}")?;
        events.sync_all()?;
        if event == "end" {
            write_json(&self.audits.join(format!("{name}.json")), &value)?;
        }
        println!(
            "Systems artifacts: {name} {event} elapsed={:.3}s",
            self.started.elapsed().as_secs_f64()
        );
        Ok(())
    }
    pub fn promote(self) -> Result<()> {
        // Never replace a prior accepted bundle. All files were synced and
        // checked while hidden in a sibling directory on this same volume.
        if self.destination.exists() {
            return Err("accepted artifact directory appeared before promotion".into());
        }
        fs::rename(&self.candidate, &self.destination)?;
        Ok(())
    }
}

pub fn issue(
    publication: CanonicalSysmlSystemsLibrary,
    sources: &VerifiedLibrarySet,
    transaction: &ArtifactTransaction,
    report: &mut Value,
) -> Result<()> {
    let cache = transaction.candidate.join("canonical.publication.zip");
    let receipt = transaction.stage("cache_serialization", || {
        let mut file = File::create(&cache)?;
        let receipt = publication.write_cache(&mut file, sources)?;
        file.sync_all()?;
        Ok(receipt)
    })?;
    transaction.stage("receipt_serialization", || {
        write_json(
            &transaction.candidate.join("accepted-publication.json"),
            &receipt,
        )
    })?;
    transaction.stage("binding_manifest_serialization", || {
        write_json(
            &transaction.candidate.join("standard-bindings.json"),
            &publication.binding_manifest(sources)?,
        )
    })?;
    // Read back the actual candidate documents, then authenticate and decode
    // the cache using the live accepted facade as the sole authority.
    let persisted_receipt = serde_json::from_reader(File::open(
        transaction.candidate.join("accepted-publication.json"),
    )?)?;
    let persisted_bindings = serde_json::from_reader(File::open(
        transaction.candidate.join("standard-bindings.json"),
    )?)?;
    let expected_identity = publication.identity().clone();
    let expected_context = publication.context().clone();
    let restored = transaction.stage("candidate_cache_restore", || {
        Ok(publication.verify_candidate_cache(
            File::open(&cache)?,
            sources,
            &persisted_receipt,
            &persisted_bindings,
        )?)
    })?;
    transaction.stage("stale_verification", || {
        restored.check_binding_manifest(sources, &persisted_bindings)?;
        if restored.identity() != &expected_identity
            || restored.context() != &expected_context
            || !restored
                .producer_closure()
                .is_fully_closed(restored.overlay().model())
        {
            return Err(
                "restored accepted artifact identity or producer certificate changed".into(),
            );
        }
        Ok(())
    })?;
    report["candidate_cache_restored"] = json!(true);
    report["candidate_restore_producers_replayed"] = json!(false);
    report["bindings_stale_check"] = json!(true);
    report["artifact_promotion"] = json!("atomic_directory_rename");
    report["exported_cache"] = json!(transaction.destination.join("canonical.publication.zip"));
    report["audit_directory"] = json!(transaction.audits);
    Ok(())
}

pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    let mut file = File::create(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_candidates_never_appear_at_the_accepted_location() {
        let root = std::env::temp_dir().join(format!("agq-issuance-{}", uuid::Uuid::new_v4()));
        let output = root.join("accepted/report.json");
        let transaction = ArtifactTransaction::begin(&output).unwrap();
        write_json(
            &transaction.candidate.join("accepted-publication.json"),
            &json!({"candidate":true}),
        )
        .unwrap();
        drop(transaction); // Models interruption before the single promotion.
        assert!(!output.parent().unwrap().exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promotion_reveals_the_whole_bundle_and_never_replaces_a_publication() {
        let root = std::env::temp_dir().join(format!("agq-issuance-{}", uuid::Uuid::new_v4()));
        let output = root.join("accepted/report.json");
        let transaction = ArtifactTransaction::begin(&output).unwrap();
        for name in [
            "report.json",
            "accepted-publication.json",
            "standard-bindings.json",
            "canonical.publication.zip",
        ] {
            write_json(&transaction.candidate.join(name), &json!({"complete":true})).unwrap();
        }
        assert!(!output.parent().unwrap().exists());
        transaction.promote().unwrap();
        assert_eq!(fs::read_dir(output.parent().unwrap()).unwrap().count(), 4);
        assert!(ArtifactTransaction::begin(&output).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_promotion_preserves_candidate_and_existing_destination() {
        let root = std::env::temp_dir().join(format!("agq-issuance-{}", uuid::Uuid::new_v4()));
        let output = root.join("accepted/report.json");
        let transaction = ArtifactTransaction::begin(&output).unwrap();
        let candidate = transaction.candidate.clone();
        fs::create_dir(output.parent().unwrap()).unwrap();
        fs::write(&output, b"existing").unwrap();
        assert!(transaction.promote().is_err());
        assert_eq!(fs::read(&output).unwrap(), b"existing");
        assert!(candidate.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
