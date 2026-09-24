//! Durable scheduler continuation, deliberately separate from accepted authority.
use super::*;
use crate::producer_closure::FrontierCertificate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const FORMAT: &str = "agq-unaccepted-publication-frontier/1";

/// Workflow-owned durable checkpoint session. The caller pins exact source bytes
/// and publication identities in `source_identity`; the scheduler additionally
/// pins its input graph, full semantic context, descriptor/producer registry,
/// options, certificate and transport reads. This type cannot issue acceptance.
///
/// `resume` requires a digest retained outside the checkpoint. Each successful
/// commit prints its immutable journal path and SHA-256 only after files are
/// synchronized. Prior completed invocations restore without producer replay.
#[derive(Debug)]
pub struct PublicationFrontierSession {
    directory: PathBuf,
    journal: Mutex<Journal>,
    cursor: AtomicUsize,
    contextual_interval: usize,
    restored_invocations: AtomicUsize,
    restored_completed_invocations: AtomicUsize,
    skipped_rounds: AtomicUsize,
    committed_checkpoints: AtomicUsize,
}

/// Evidence that a resumed workflow actually restored scheduler work. These
/// observations never participate in graph identity or publication acceptance.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicationFrontierStatistics {
    pub restored_invocations: usize,
    pub restored_completed_invocations: usize,
    pub skipped_rounds: usize,
    pub committed_checkpoints: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Journal {
    format: String,
    source_identity: [u8; 32],
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry {
    input_identity: [u8; 32],
    archive_sha256: [u8; 32],
    state_sha256: [u8; 32],
    graph_sha256: [u8; 32],
}

/// Exact state at the start of the next round; the graph contains all writes
/// from the completed frontier. Observational timing is never resume authority.
#[derive(Serialize, Deserialize)]
pub(super) struct State<Certificate = FrontierCertificate> {
    pub round: usize,
    pub converged: bool,
    pub stratum: ResultStructureStratum,
    pub model_digest: [u8; 32],
    pub context_contract: [u8; 32],
    pub stages: Vec<PublicationStage>,
    pub counters: PublicationCounters,
    pub status: BTreeMap<ElementId, (Completeness, BTreeSet<Diagnostic>)>,
    pub population: BTreeSet<ElementId>,
    pub worklist: BTreeSet<ElementId>,
    pub seen: BTreeSet<ElementId>,
    pub deferred_bindings: BTreeSet<ElementId>,
    pub dependencies: BTreeMap<ElementId, Box<[InvalidationKey]>>,
    pub certificate: Certificate,
    /// Evaluations are separately retained: certificate causal closure can mark
    /// an evaluated row pending without discarding its recorded evaluation.
    pub evaluation_rows: Vec<(ElementId, Vec<u8>)>,
}

/// Authenticated strict graph and scheduler state, still without publication authority.
/// Fields are private: convergence and closure cannot be supplied by callers.
pub struct ConvergedPublicationFrontier {
    overlay: DerivedOverlay,
    state: State,
}

impl ConvergedPublicationFrontier {
    /// Exact graph decoded under the independently supplied immutable dependency.
    pub fn overlay(&self) -> &DerivedOverlay {
        &self.overlay
    }

    /// Authenticate the final semantic context and complete closure coverage.
    /// This performs no producer evaluation, graph mutation or context rebasing.
    pub fn authenticate(
        self,
        mut context_factory: impl for<'m> FnMut(
            &'m DerivedOverlay,
        )
            -> Result<SemanticContext<'m>, PublicationOverlayError>,
        registry: &ProducerRegistry,
    ) -> Result<PublicationClosure, PublicationOverlayError> {
        let Self { overlay, state } = self;
        let bound_context = context_factory(&overlay)?;
        if !std::ptr::eq(bound_context.model(), overlay.model()) {
            return Err(failure(
                "converged frontier context is bound to another model",
            ));
        }
        let context = bound_context.id().clone();
        drop(bound_context);
        if context.model_digest != state.model_digest
            || context.closure_contract_digest() != state.context_contract
            || context.producer_registry_digest != Some(registry.digest())
        {
            return Err(failure(
                "converged frontier graph/context/registry mismatch",
            ));
        }
        let certificate =
            restore_certificate(state.certificate, &context, registry, overlay.model())?;
        if !certificate.is_fully_closed(overlay.model())
            || certificate.applicable_pairs() != certificate.closed_pairs()
            || certificate.incomplete_pairs() != 0
        {
            return Err(failure("converged frontier closure coverage incomplete"));
        }
        if state.counters.applicable_subject_family_pairs != certificate.applicable_pairs()
            || state.counters.closed_producer_pairs != certificate.closed_pairs()
            || state.counters.incomplete_producer_pairs != certificate.incomplete_pairs()
            || state.counters.closed_producer_effects != certificate.closed_effects()
            || state.counters.families_registered != registry.descriptors().len()
        {
            return Err(failure(
                "converged frontier certificate accounting mismatch",
            ));
        }
        // The separately retained scheduler rows must agree with its certificate.
        // Inspect bytes directly without cloning its potentially large proof trees.
        let mut seen = BTreeSet::new();
        for (subject, row) in &state.evaluation_rows {
            if !seen.insert(*subject)
                || row.len() != registry.descriptors().len()
                || row.iter().enumerate().any(|(family, value)| {
                    certificate
                        .evaluation(*subject, family)
                        .map(|state| state as u8)
                        != Some(*value)
                })
            {
                return Err(failure("converged frontier evaluation rows mismatch"));
            }
        }
        let local = overlay
            .model()
            .elements()
            .filter(|record| !overlay.declared().is_dependency_element(record.id()))
            .map(|record| record.id())
            .collect();
        if seen != state.population || seen != local {
            return Err(failure("converged frontier evaluation coverage mismatch"));
        }
        Ok(PublicationClosure {
            overlay,
            stages: state.stages,
            counters: state.counters,
            completeness: Completeness::Complete,
            converged: true,
            certificate: Some(certificate),
            producer_reads: QueryInvalidationSet::from_keys(
                state.dependencies.into_values().flatten().collect(),
            ),
        })
    }
}

pub(super) struct Invocation<'a> {
    session: &'a PublicationFrontierSession,
    index: usize,
    identity: [u8; 32],
    previous: Option<Entry>,
}

fn failure(error: impl std::fmt::Display) -> PublicationOverlayError {
    PublicationOverlayError::FrontierCheckpoint(error.to_string())
}

fn digest_file(path: &Path) -> Result<[u8; 32], PublicationOverlayError> {
    let mut file = File::open(path).map_err(failure)?;
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut bytes).map_err(failure)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    Ok(hash.finalize().into())
}

fn hex(digest: [u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct HashedIo<T> {
    inner: T,
    hash: Sha256,
}
impl<T> HashedIo<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            hash: Sha256::new(),
        }
    }
    fn digest(self) -> [u8; 32] {
        self.hash.finalize().into()
    }
}
impl<T: Write> Write for HashedIo<T> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self.inner.write(bytes)?;
        self.hash.update(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
impl<T: Read> Read for HashedIo<T> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(bytes)?;
        self.hash.update(&bytes[..count]);
        Ok(count)
    }
}

impl PublicationFrontierSession {
    /// Create a fresh session; zero interval checkpoints only completed strata
    /// and final invocations. Nonzero additionally captures contextual rounds.
    pub fn create(
        directory: impl Into<PathBuf>,
        source_identity: [u8; 32],
        contextual_interval: usize,
    ) -> Result<Self, PublicationOverlayError> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory).map_err(failure)?;
        Ok(Self {
            directory,
            journal: Mutex::new(Journal {
                format: FORMAT.into(),
                source_identity,
                entries: vec![],
            }),
            cursor: AtomicUsize::new(0),
            contextual_interval,
            restored_invocations: AtomicUsize::new(0),
            restored_completed_invocations: AtomicUsize::new(0),
            skipped_rounds: AtomicUsize::new(0),
            committed_checkpoints: AtomicUsize::new(0),
        })
    }

    /// Authenticate the immutable journal before interpreting any checkpoint
    /// state. A changed source/dependency contract or digest is rejected.
    pub fn resume(
        journal: impl AsRef<Path>,
        expected_sha256: [u8; 32],
        source_identity: [u8; 32],
        contextual_interval: usize,
    ) -> Result<Self, PublicationOverlayError> {
        let path = journal.as_ref();
        let bytes = std::fs::read(path).map_err(failure)?;
        if <[u8; 32]>::from(Sha256::digest(&bytes)) != expected_sha256 {
            return Err(failure("frontier journal authentication mismatch"));
        }
        let journal: Journal = serde_json::from_slice(&bytes).map_err(failure)?;
        if journal.format != FORMAT || journal.source_identity != source_identity {
            return Err(failure("frontier source/publication identity mismatch"));
        }
        Ok(Self {
            directory: path
                .parent()
                .ok_or_else(|| failure("frontier journal parent"))?
                .to_owned(),
            journal: Mutex::new(journal),
            cursor: AtomicUsize::new(0),
            contextual_interval,
            restored_invocations: AtomicUsize::new(0),
            restored_completed_invocations: AtomicUsize::new(0),
            skipped_rounds: AtomicUsize::new(0),
            committed_checkpoints: AtomicUsize::new(0),
        })
    }

    /// Latest durable journal pin. Retain this tuple independently of the files
    /// before interruption; callers must never trust a pin from edited bytes.
    pub fn latest_checkpoint(
        &self,
    ) -> Result<Option<(PathBuf, [u8; 32])>, PublicationOverlayError> {
        let journal = self.journal.lock().map_err(failure)?;
        if journal.entries.is_empty() {
            return Ok(None);
        }
        let digest: [u8; 32] =
            Sha256::digest(serde_json::to_vec(&*journal).map_err(failure)?).into();
        Ok(Some((
            self.directory.join(format!("journal-{}.json", hex(digest))),
            digest,
        )))
    }

    pub fn statistics(&self) -> PublicationFrontierStatistics {
        PublicationFrontierStatistics {
            restored_invocations: self.restored_invocations.load(Ordering::Relaxed),
            restored_completed_invocations: self
                .restored_completed_invocations
                .load(Ordering::Relaxed),
            skipped_rounds: self.skipped_rounds.load(Ordering::Relaxed),
            committed_checkpoints: self.committed_checkpoints.load(Ordering::Relaxed),
        }
    }

    /// Reject reuse for different independently verified input bytes.
    pub fn validate_source_identity(
        &self,
        expected: [u8; 32],
    ) -> Result<(), PublicationOverlayError> {
        if self.journal.lock().map_err(failure)?.source_identity != expected {
            return Err(failure("frontier source/publication identity mismatch"));
        }
        Ok(())
    }

    /// Decode the final strict checkpoint only. Earlier construction invocations
    /// are neither replayed nor interpreted. The authenticated journal transitively
    /// pins graph bytes, certificate bytes and exact proof/search transport bytes.
    pub fn restore_converged_frontier(
        &self,
        registry: std::sync::Arc<agq_kernel::metamodel::MetamodelRegistry>,
        dependency: Option<std::sync::Arc<DerivedOverlay>>,
    ) -> Result<ConvergedPublicationFrontier, PublicationOverlayError> {
        let entry = self
            .journal
            .lock()
            .map_err(failure)?
            .entries
            .last()
            .cloned()
            .ok_or_else(|| failure("missing converged frontier"))?;
        let path = self
            .directory
            .join(format!("{}.zip", hex(entry.archive_sha256)));
        if digest_file(&path)? != entry.archive_sha256 {
            return Err(failure("frontier archive authentication mismatch"));
        }
        let mut archive = ZipArchive::new(File::open(path).map_err(failure)?).map_err(failure)?;
        if archive.len() != 2 {
            return Err(failure("frontier archive entry count"));
        }
        let mut state_reader = HashedIo::new(archive.by_name("state.json").map_err(failure)?);
        let state: State = serde_json::from_reader(&mut state_reader).map_err(failure)?;
        if state_reader.digest() != entry.state_sha256 {
            return Err(failure("decoded frontier state authentication mismatch"));
        }
        if !state.converged
            || !state.worklist.is_empty()
            || state.stratum != ResultStructureStratum::ContextualBindings
            || state
                .status
                .values()
                .any(|(status, _)| *status != Completeness::Complete)
            || state
                .stages
                .last()
                .is_none_or(|stage| stage.completeness != Completeness::Complete)
        {
            return Err(failure("frontier is not strictly converged"));
        }
        let mut graph_reader = HashedIo::new(archive.by_name("graph.jsonl").map_err(failure)?);
        let overlay = agq_kernel::archive::read_publication_frontier(
            BufReader::new(&mut graph_reader),
            registry,
            dependency,
        )
        .map_err(failure)?;
        if graph_reader.digest() != entry.graph_sha256 {
            return Err(failure("decoded frontier graph authentication mismatch"));
        }
        self.restored_invocations.fetch_add(1, Ordering::Relaxed);
        self.restored_completed_invocations
            .fetch_add(1, Ordering::Relaxed);
        self.skipped_rounds
            .fetch_add(state.round, Ordering::Relaxed);
        Ok(ConvergedPublicationFrontier { overlay, state })
    }

    pub(super) fn begin(
        &self,
        context: &SemanticContextId,
        options: &PublicationClosureOptions,
        stable_properties: bool,
    ) -> Result<Invocation<'_>, PublicationOverlayError> {
        let index = self.cursor.fetch_add(1, Ordering::SeqCst);
        let mut hash = Sha256::new();
        hash.update(FORMAT);
        hash.update(context.model_digest);
        hash.update(context.closure_contract_digest());
        // These govern pending populations and must match the exact continuation.
        hash.update(format!(
            "{:?}/{:?}/{:?}/{}/{}/{stable_properties}",
            options.initial_subjects,
            options.strategy,
            options.order,
            options.batch_size,
            options.max_rounds
        ));
        let identity = hash.finalize().into();
        let journal = self.journal.lock().map_err(failure)?;
        let previous = journal.entries.get(index).cloned();
        if previous
            .as_ref()
            .is_some_and(|entry| entry.input_identity != identity)
        {
            return Err(failure("frontier initial graph/context/options mismatch"));
        }
        Ok(Invocation {
            session: self,
            index,
            identity,
            previous,
        })
    }
}

impl Invocation<'_> {
    pub(super) fn has_saved_frontier(&self) -> bool {
        self.previous.is_some()
    }
    pub(super) fn should_capture(
        &self,
        round: usize,
        stratum: ResultStructureStratum,
        prior: Option<ResultStructureStratum>,
        converged: bool,
    ) -> bool {
        converged
            || (round > 0 && prior != Some(stratum))
            || (stratum == ResultStructureStratum::ContextualBindings
                && self.session.contextual_interval > 0
                && round.is_multiple_of(self.session.contextual_interval))
    }

    pub(super) fn restore<O: ProducerFrontier>(
        &self,
        input: &O::Input,
    ) -> Result<Option<(O, State)>, PublicationOverlayError> {
        let Some(entry) = &self.previous else {
            return Ok(None);
        };
        let path = self
            .session
            .directory
            .join(format!("{}.zip", hex(entry.archive_sha256)));
        if digest_file(&path)? != entry.archive_sha256 {
            return Err(failure("frontier archive authentication mismatch"));
        }
        let mut archive = ZipArchive::new(File::open(path).map_err(failure)?).map_err(failure)?;
        if archive.len() != 2 {
            return Err(failure("frontier archive entry count"));
        }
        let mut state_reader = HashedIo::new(archive.by_name("state.json").map_err(failure)?);
        let state: State = serde_json::from_reader(&mut state_reader).map_err(failure)?;
        if state_reader.digest() != entry.state_sha256 {
            return Err(failure("decoded frontier state authentication mismatch"));
        }
        let mut graph_reader = HashedIo::new(archive.by_name("graph.jsonl").map_err(failure)?);
        let overlay =
            O::read_frontier(&mut BufReader::new(&mut graph_reader), input).map_err(failure)?;
        if graph_reader.digest() != entry.graph_sha256 {
            return Err(failure("decoded frontier graph authentication mismatch"));
        }
        Ok(Some((overlay, state)))
    }

    pub(super) fn restored(&self, round: usize, stratum: ResultStructureStratum, converged: bool) {
        self.session
            .restored_invocations
            .fetch_add(1, Ordering::Relaxed);
        self.session
            .restored_completed_invocations
            .fetch_add(usize::from(converged), Ordering::Relaxed);
        self.session
            .skipped_rounds
            .fetch_add(round, Ordering::Relaxed);
        eprintln!(
            "Publication checkpoint restored: invocation={} round={} stratum={:?} converged={}",
            self.index, round, stratum, converged
        );
    }

    pub(super) fn capture<O: ProducerFrontier, C: Serialize>(
        &self,
        overlay: &O,
        state: &State<C>,
    ) -> Result<(), PublicationOverlayError> {
        static TEMP: AtomicUsize = AtomicUsize::new(0);
        let temp = self.session.directory.join(format!(
            "frontier-{}-{}.tmp",
            std::process::id(),
            TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&temp)
            .map_err(failure)?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .large_file(true);
        archive.start_file("state.json", options).map_err(failure)?;
        let mut state_writer = HashedIo::new(&mut archive);
        serde_json::to_writer(&mut state_writer, state).map_err(failure)?;
        let state_sha256 = state_writer.digest();
        archive
            .start_file("graph.jsonl", options)
            .map_err(failure)?;
        let mut graph_writer = HashedIo::new(&mut archive);
        overlay.write_frontier(&mut graph_writer).map_err(failure)?;
        let graph_sha256 = graph_writer.digest();
        archive
            .finish()
            .map_err(failure)?
            .sync_all()
            .map_err(failure)?;
        let archive_sha256 = digest_file(&temp)?;
        let destination = self
            .session
            .directory
            .join(format!("{}.zip", hex(archive_sha256)));
        if destination.exists() {
            if digest_file(&destination)? != archive_sha256 {
                return Err(failure("frontier immutable object collision"));
            }
            std::fs::remove_file(&temp).map_err(failure)?;
        } else {
            std::fs::rename(&temp, &destination).map_err(failure)?;
        }
        // Do not mutate the active in-memory journal until the new complete
        // journal has been durably written. A failed write leaves the old pin valid.
        let mut journal = self.session.journal.lock().map_err(failure)?;
        let mut next = journal.clone();
        let entry = Entry {
            input_identity: self.identity,
            archive_sha256,
            state_sha256,
            graph_sha256,
        };
        if next.entries.len() == self.index {
            next.entries.push(entry);
        } else if let Some(previous) = next.entries.get_mut(self.index) {
            *previous = entry;
        } else {
            return Err(failure("frontier invocation order"));
        }
        let bytes = serde_json::to_vec(&next).map_err(failure)?;
        let journal_digest: [u8; 32] = Sha256::digest(&bytes).into();
        let path = self
            .session
            .directory
            .join(format!("journal-{}.json", hex(journal_digest)));
        if !path.exists() {
            let pending = self.session.directory.join(format!(
                "journal-{}-{}.tmp",
                std::process::id(),
                TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending)
                .map_err(failure)?;
            file.write_all(&bytes).map_err(failure)?;
            file.sync_all().map_err(failure)?;
            drop(file);
            std::fs::rename(&pending, &path).map_err(failure)?;
        } else if digest_file(&path)? != journal_digest {
            return Err(failure("frontier immutable journal collision"));
        }
        *journal = next;
        self.session
            .committed_checkpoints
            .fetch_add(1, Ordering::Relaxed);
        eprintln!(
            "Publication checkpoint committed: journal={} sha256={} invocation={} round={} stratum={:?}",
            path.display(),
            hex(journal_digest),
            self.index,
            state.round,
            state.stratum
        );
        Ok(())
    }
}

pub(super) fn restore_certificate(
    state: FrontierCertificate,
    context: &SemanticContextId,
    registry: &ProducerRegistry,
    model: &ModelView,
) -> Result<std::sync::Arc<ProducerClosureCertificate>, PublicationOverlayError> {
    ProducerClosureCertificate::restore_frontier_state(state, context, registry, model)
        .map(std::sync::Arc::new)
        .ok_or_else(|| failure("frontier closure certificate mismatch"))
}
