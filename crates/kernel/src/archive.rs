//! Streaming, versioned kernel graph archives. Loading proves structural integrity,
//! never language publication, rule truth, or authority acceptance.
//!
//! Root archives contain one declared snapshot and optionally its derived overlay.
//! Dependent archives contain only a local delta and pin the separately supplied
//! immutable dependency. Compression and trusted content pins belong to the
//! caller. A caller can wrap any `Read`/`Write` in its own compression stream.
use super::*;
use crate::derived::{ComputationFailure, DerivedOverlay, IncompleteReason, StructuralSearch};
use crate::provenance::{Explanation, ExplanationPool};
use crate::shared_map::{SharedMap, SharedSet};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, Write};

const FORMAT: &str = "agq-kernel-graph-archive/1";
const DEPENDENT_FORMAT: &str = "agq-kernel-dependent-graph-archive/1";
const DEPENDENT_EVIDENCE_FORMAT: &str = "agq-kernel-dependent-evidence-archive/1";
const MAX_LINE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
enum ArchiveFormat {
    Legacy,
    Frontier,
    DependentEvidence,
    BoundFrontier([u8; 32]),
}
impl ArchiveFormat {
    fn identity(self, construction: bool, dependent: bool) -> &'static str {
        match self {
            Self::Legacy if dependent => DEPENDENT_FORMAT,
            Self::Legacy => FORMAT,
            Self::Frontier => frontier_format(construction, dependent),
            Self::DependentEvidence => DEPENDENT_EVIDENCE_FORMAT,
            Self::BoundFrontier(_) => "agq-kernel-bound-publication-frontier/1",
        }
    }
    fn preserves_contributions(self) -> bool {
        !matches!(self, Self::Legacy)
    }
}

/// Serialization, registry identity, or structural reconstruction failed.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Derivation(#[from] crate::derived::DerivationError),
    #[error("invalid kernel graph archive: {0}")]
    Invalid(&'static str),
}

#[derive(Serialize, Deserialize)]
enum Entry {
    Header {
        format: String,
        registry: [u8; 32],
        overlay: bool,
    },
    DependentHeader {
        format: String,
        registry: [u8; 32],
        dependency: [u8; 32],
    },
    Proof(Explanation),
    Search(BTreeSet<StructuralSearch>),
    Snapshot {
        revision: RevisionId,
        used_ids: BTreeSet<ElementId>,
        used_links: BTreeSet<AssociationOccurrenceId>,
    },
    Record(Record),
    Occurrence(Occurrence),
    Overlay,
    Navigation {
        element: ElementId,
        property: PropertyId,
        slot: StoredSlot,
    },
    Failure {
        element: ElementId,
        property: PropertyId,
        failure: Failure,
    },
    Searches {
        fact: FactKey,
        table: usize,
    },
    /// Present in lossless dependent evidence and producer-frontier formats.
    ReferenceContribution {
        element: ElementId,
        property: PropertyId,
        target: ElementId,
        position: usize,
        proof: usize,
        searches: usize,
    },
    End,
}
#[derive(Serialize, Deserialize)]
struct Record {
    id: ElementId,
    class: MetaclassId,
    origin: StoredOrigin,
    slots: Vec<(PropertyId, StoredSlot)>,
}
#[derive(Serialize, Deserialize)]
struct Occurrence {
    id: AssociationOccurrenceId,
    association: AssociationId,
    ends: Vec<(PropertyId, ElementId)>,
    positions: Vec<(PropertyId, usize)>,
    origin: StoredOrigin,
}
#[derive(Serialize, Deserialize)]
enum StoredOrigin {
    Declared(DeclaredOrigin),
    Proof(usize),
}
#[derive(Serialize, Deserialize)]
struct StoredSlot {
    value: StoredValue,
    origin: StoredOrigin,
}
#[derive(Serialize, Deserialize)]
enum StoredValue {
    Scalar(Scalar),
    Ordered(Vec<Scalar>),
    Set(Vec<Scalar>),
    Bag(Vec<Scalar>),
}
#[derive(Serialize, Deserialize)]
enum Scalar {
    Boolean(bool),
    Integer(String),
    Real(String),
    String(String),
    Enumeration(crate::EnumerationLiteralId),
    Reference(ElementId),
}
#[derive(Serialize, Deserialize)]
enum Failure {
    Incomplete {
        reason: IncompleteReason,
        proof: usize,
        searches: usize,
    },
    Invalid {
        diagnostic: String,
        proof: usize,
        searches: usize,
    },
}
impl From<&Value> for Scalar {
    fn from(value: &Value) -> Self {
        match value {
            Value::Boolean(v) => Self::Boolean(*v),
            Value::Integer(v) => Self::Integer(v.to_string()),
            Value::Real(v) => Self::Real(v.to_string()),
            Value::String(v) => Self::String(v.clone()),
            Value::Enumeration(v) => Self::Enumeration(*v),
            Value::Reference(v) => Self::Reference(*v),
        }
    }
}
impl Scalar {
    fn restore(self) -> Result<Value, ArchiveError> {
        Ok(match self {
            Self::Boolean(v) => Value::Boolean(v),
            Self::String(v) => Value::String(v),
            Self::Enumeration(v) => Value::Enumeration(v),
            Self::Reference(v) => Value::Reference(v),
            Self::Integer(v) => {
                Value::Integer(v.parse().map_err(|_| ArchiveError::Invalid("integer"))?)
            }
            Self::Real(v) => Value::Real(v.parse().map_err(|_| ArchiveError::Invalid("decimal"))?),
        })
    }
}
impl From<&SlotValue> for StoredValue {
    fn from(value: &SlotValue) -> Self {
        let values = || value.values().map(Scalar::from).collect();
        match value {
            SlotValue::Scalar(v) => Self::Scalar(v.into()),
            SlotValue::Ordered(_) => Self::Ordered(values()),
            SlotValue::Set(_) => Self::Set(values()),
            SlotValue::Bag(_) => Self::Bag(values()),
        }
    }
}
impl StoredValue {
    fn restore(self) -> Result<SlotValue, ArchiveError> {
        let convert = |values: Vec<Scalar>| {
            values
                .into_iter()
                .map(Scalar::restore)
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(match self {
            Self::Scalar(v) => SlotValue::Scalar(v.restore()?),
            Self::Ordered(v) => SlotValue::Ordered(convert(v)?),
            Self::Set(v) => {
                let values = convert(v)?;
                let set: BTreeSet<_> = values.iter().cloned().collect();
                if values.len() != set.len() {
                    return Err(ArchiveError::Invalid("duplicate set value"));
                }
                SlotValue::Set(set)
            }
            Self::Bag(v) => {
                let values = convert(v)?;
                if !values.is_sorted() {
                    return Err(ArchiveError::Invalid("noncanonical bag"));
                }
                SlotValue::Bag(values)
            }
        })
    }
}

// Addresses are only a fast path. Equal independently allocated sets get one
// table entry too. Table numbers follow deterministic graph traversal order.
struct Tables<'a> {
    proof_addresses: HashMap<usize, usize>,
    proofs: HashMap<&'a Explanation, usize>,
    search_addresses: HashMap<usize, usize>,
    searches: HashMap<&'a BTreeSet<StructuralSearch>, usize>,
}
impl<'a> Tables<'a> {
    fn new() -> Self {
        Self {
            proof_addresses: HashMap::new(),
            proofs: HashMap::new(),
            search_addresses: HashMap::new(),
            searches: HashMap::new(),
        }
    }
    fn proof(
        &mut self,
        proof: &'a Explanation,
        writer: &mut impl Write,
    ) -> Result<usize, ArchiveError> {
        let address = std::ptr::from_ref(proof) as usize;
        if let Some(&index) = self.proof_addresses.get(&address) {
            return Ok(index);
        }
        let index = if let Some(&index) = self.proofs.get(proof) {
            index
        } else {
            let index = self.proofs.len();
            emit(writer, &Entry::Proof(proof.clone()))?;
            self.proofs.insert(proof, index);
            index
        };
        self.proof_addresses.insert(address, index);
        Ok(index)
    }
    fn search(
        &mut self,
        searches: &'a BTreeSet<StructuralSearch>,
        writer: &mut impl Write,
    ) -> Result<usize, ArchiveError> {
        let address = std::ptr::from_ref(searches) as usize;
        if let Some(&index) = self.search_addresses.get(&address) {
            return Ok(index);
        }
        let index = if let Some(&index) = self.searches.get(searches) {
            index
        } else {
            let index = self.searches.len();
            emit(writer, &Entry::Search(searches.clone()))?;
            self.searches.insert(searches, index);
            index
        };
        self.search_addresses.insert(address, index);
        Ok(index)
    }
    fn origin(
        &mut self,
        origin: &'a Origin,
        writer: &mut impl Write,
    ) -> Result<StoredOrigin, ArchiveError> {
        match origin {
            Origin::Declared(v) => Ok(StoredOrigin::Declared(v.clone())),
            Origin::Derived(v) => Ok(StoredOrigin::Proof(self.proof(v, writer)?)),
            Origin::AssociationOccurrences(_) => Err(ArchiveError::Invalid(
                "projected origin stored as canonical fact",
            )),
        }
    }
    fn slot(
        &mut self,
        slot: &'a Slot,
        writer: &mut impl Write,
    ) -> Result<StoredSlot, ArchiveError> {
        Ok(StoredSlot {
            value: (&slot.value).into(),
            origin: self.origin(&slot.origin, writer)?,
        })
    }
    fn record(
        &mut self,
        record: &'a ElementRecord,
        writer: &mut impl Write,
    ) -> Result<(), ArchiveError> {
        let origin = self.origin(&record.origin, writer)?;
        let slots = record
            .slots
            .iter()
            .map(|(&property, slot)| self.slot(slot, writer).map(|slot| (property, slot)))
            .collect::<Result<_, _>>()?;
        emit(
            writer,
            &Entry::Record(Record {
                id: record.id,
                class: record.metaclass,
                origin,
                slots,
            }),
        )
    }
    fn occurrence(
        &mut self,
        occurrence: &'a AssociationOccurrence,
        writer: &mut impl Write,
    ) -> Result<(), ArchiveError> {
        let origin = self.origin(&occurrence.origin, writer)?;
        emit(
            writer,
            &Entry::Occurrence(Occurrence {
                id: occurrence.id,
                association: occurrence.association,
                ends: occurrence.ends.iter().map(|(&a, &b)| (a, b)).collect(),
                positions: occurrence.positions.iter().map(|(&a, &b)| (a, b)).collect(),
                origin,
            }),
        )
    }
}
fn emit(writer: &mut impl Write, entry: &Entry) -> Result<(), ArchiveError> {
    serde_json::to_writer(&mut *writer, entry)?;
    writer.write_all(b"\n")?;
    Ok(())
}
// This private archive version is tied to the registry's complete deterministic
// Debug encoding, matching the current semantic descriptor identity convention.
fn registry_digest(registry: &MetamodelRegistry) -> [u8; 32] {
    Sha256::digest(format!("{registry:?}").as_bytes()).into()
}

/// Stream one declared root snapshot. This does not certify language conformance.
pub fn write_snapshot(snapshot: &Snapshot, writer: impl Write) -> Result<(), ArchiveError> {
    write(snapshot, None, false, writer)
}
/// Stream the historical root graph format with shared aggregate evidence tables.
/// Selected ordered-reference contribution caches are omitted by this encoding.
pub fn write_overlay(overlay: &DerivedOverlay, writer: impl Write) -> Result<(), ArchiveError> {
    write(overlay.declared(), Some(overlay), false, writer)
}
/// Stream only the local graph delta over one exact immutable dependency.
/// The dependency must have its own root archive; this confers no language seal.
/// This historical encoding omits local selected contributions; use
/// [`write_dependent_overlay_with_evidence`] when their exact support is required.
pub fn write_dependent_overlay(
    overlay: &DerivedOverlay,
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write(overlay.declared(), Some(overlay), true, writer)
}

/// Stream a strict local graph delta including selected ordered-reference proofs
/// and searches. The separately supplied immutable dependency remains external.
/// This lossless format contains no scheduler state or publication authority.
/// Historical root/dependent archive encodings remain unchanged.
pub fn write_dependent_overlay_with_evidence(
    overlay: &DerivedOverlay,
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write_input(
        &DerivationInput::Strict(overlay.declared().clone()),
        Some(overlay.model()),
        true,
        ArchiveFormat::DependentEvidence,
        writer,
    )
}

/// Stream an unaccepted strict producer frontier, preserving selected ordered
/// contribution evidence. This separate format is never a publication cache.
pub fn write_publication_frontier(
    overlay: &DerivedOverlay,
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write_input(
        &DerivationInput::Strict(overlay.declared().clone()),
        Some(overlay.model()),
        overlay.declared().immutable_dependency().is_some(),
        ArchiveFormat::Frontier,
        writer,
    )
}

/// Stream only local strict facts and reservations over an independently
/// authenticated dependency, including a dependency that itself has protected
/// layers. The caller supplies that dependency's exact content identity. It must
/// authenticate the same identity before using [`read_bound_frontier_on`].
/// This format neither serializes nor flattens any protected dependency and
/// confers no publication or language authority. Historical formats are unchanged.
pub fn write_bound_frontier(
    overlay: &DerivedOverlay,
    dependency_identity: [u8; 32],
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write_input(
        &DerivationInput::Strict(overlay.declared().clone()),
        Some(overlay.model()),
        true,
        ArchiveFormat::BoundFrontier(dependency_identity),
        writer,
    )
}

/// Stream an unpublished construction frontier, including missing lower bounds.
/// Every declaration, inferred proof/search and ordered contribution is retained.
pub fn write_construction_frontier(
    overlay: &crate::derived::ConstructionOverlay,
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write_input(
        &DerivationInput::Construction(overlay.declared_shared().clone()),
        Some(overlay.model()),
        overlay.declared().immutable_dependency().is_some(),
        ArchiveFormat::Frontier,
        writer,
    )
}

fn frontier_format(construction: bool, dependent: bool) -> &'static str {
    match (construction, dependent) {
        (false, false) => "agq-kernel-publication-frontier/1",
        (false, true) => "agq-kernel-dependent-publication-frontier/1",
        (true, false) => "agq-kernel-construction-frontier/1",
        (true, true) => "agq-kernel-dependent-construction-frontier/1",
    }
}

struct ArchiveDigest(Sha256);
impl Write for ArchiveDigest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn dependency_digest(
    dependency: &DerivedOverlay,
    format: ArchiveFormat,
) -> Result<[u8; 32], ArchiveError> {
    if let ArchiveFormat::BoundFrontier(identity) = format {
        let mut hash = Sha256::new();
        hash.update(b"agq-kernel-bound-frontier-dependency/1\0");
        hash.update(identity);
        return Ok(hash.finalize().into());
    }
    let mut writer = ArchiveDigest(Sha256::new());
    if matches!(format, ArchiveFormat::DependentEvidence) {
        writer.write_all(b"agq-kernel-dependent-evidence-dependency/1\0")?;
    }
    write_overlay(dependency, &mut writer)?;
    if matches!(format, ArchiveFormat::DependentEvidence) {
        for (key, contribution) in dependency.model().ordered_reference_contributions() {
            serde_json::to_writer(
                &mut writer,
                &(
                    key,
                    contribution.position(),
                    contribution.explanation(),
                    contribution.searches(),
                ),
            )?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(writer.0.finalize().into())
}
fn write(
    snapshot: &Snapshot,
    overlay: Option<&DerivedOverlay>,
    dependent: bool,
    writer: impl Write,
) -> Result<(), ArchiveError> {
    write_input(
        &DerivationInput::Strict(snapshot.clone()),
        overlay.map(DerivedOverlay::model),
        dependent,
        ArchiveFormat::Legacy,
        writer,
    )
}

fn write_input(
    snapshot: &DerivationInput,
    overlay: Option<&ModelView>,
    dependent: bool,
    format: ArchiveFormat,
    mut writer: impl Write,
) -> Result<(), ArchiveError> {
    let dependency = snapshot.immutable_dependency();
    if dependent {
        let dependency = dependency.ok_or(ArchiveError::Invalid("missing immutable dependency"))?;
        emit(
            &mut writer,
            &Entry::DependentHeader {
                format: format
                    .identity(matches!(snapshot, DerivationInput::Construction(_)), true)
                    .into(),
                registry: registry_digest(snapshot.model().registry()),
                dependency: dependency_digest(dependency, format)?,
            },
        )?;
    } else if dependency.is_some() || snapshot.model().declared_source.is_some() {
        return Err(ArchiveError::Invalid(
            "protected dependency snapshots require their own archive boundary",
        ));
    } else {
        emit(
            &mut writer,
            &Entry::Header {
                format: format
                    .identity(matches!(snapshot, DerivationInput::Construction(_)), false)
                    .into(),
                registry: registry_digest(snapshot.model().registry()),
                overlay: overlay.is_some(),
            },
        )?;
    }
    emit(
        &mut writer,
        &Entry::Snapshot {
            revision: snapshot.revision(),
            used_ids: match snapshot {
                DerivationInput::Strict(v) => &v.inner.used_ids,
                DerivationInput::Construction(v) => &v.used_ids,
            }
            .iter()
            .copied()
            .filter(|id| {
                !matches!(format, ArchiveFormat::BoundFrontier(_))
                    || dependency.is_none_or(|d| !d.element_reservations().contains(id))
            })
            .collect(),
            used_links: match snapshot {
                DerivationInput::Strict(v) => &v.inner.used_links,
                DerivationInput::Construction(v) => &v.used_links,
            }
            .iter()
            .copied()
            .filter(|id| {
                !matches!(format, ArchiveFormat::BoundFrontier(_))
                    || dependency.is_none_or(|d| !d.occurrence_reservations().contains(id))
            })
            .collect(),
        },
    )?;
    let mut tables = Tables::new();
    for record in snapshot.model().elements() {
        if dependency.is_none_or(|d| d.model().element(record.id()).is_none()) {
            tables.record(record, &mut writer)?;
        }
    }
    for occurrence in snapshot.model().association_occurrences() {
        if dependency.is_none_or(|d| d.model().association_occurrence(occurrence.id()).is_none()) {
            tables.occurrence(occurrence, &mut writer)?;
        }
    }
    if let Some(overlay) = overlay {
        emit(&mut writer, &Entry::Overlay)?;
        for record in overlay.elements() {
            if snapshot.model().element(record.id()) != Some(record) {
                tables.record(record, &mut writer)?;
            }
        }
        for occurrence in overlay.association_occurrences() {
            if snapshot.model().association_occurrence(occurrence.id()) != Some(occurrence) {
                tables.occurrence(occurrence, &mut writer)?;
            }
        }
        for ((element, property), slot) in overlay.derived_navigation_results() {
            if dependency
                .is_some_and(|d| d.model().navigation_slot(*element, *property) == Some(slot))
            {
                continue;
            }
            let slot = tables.slot(slot, &mut writer)?;
            emit(
                &mut writer,
                &Entry::Navigation {
                    element: *element,
                    property: *property,
                    slot,
                },
            )?;
        }
        for (&(element, property), failure) in &overlay.statuses {
            if dependency
                .is_some_and(|d| d.model().statuses.get(&(element, property)) == Some(failure))
            {
                continue;
            }
            let failure = match failure {
                ComputationFailure::Incomplete {
                    reason,
                    explanation,
                    searches,
                } => Failure::Incomplete {
                    reason: reason.clone(),
                    proof: tables.proof(explanation, &mut writer)?,
                    searches: tables.search(searches, &mut writer)?,
                },
                ComputationFailure::Invalid {
                    diagnostic,
                    explanation,
                    searches,
                } => Failure::Invalid {
                    diagnostic: diagnostic.clone(),
                    proof: tables.proof(explanation, &mut writer)?,
                    searches: tables.search(searches, &mut writer)?,
                },
            };
            emit(
                &mut writer,
                &Entry::Failure {
                    element,
                    property,
                    failure,
                },
            )?;
        }
        for (fact, searches) in overlay.computation_searches() {
            if dependency.is_some_and(|d| {
                d.model()
                    .computation_searches_shared(*fact)
                    .is_some_and(|before| before.as_ref() == searches)
            }) {
                continue;
            }
            let table = tables.search(searches, &mut writer)?;
            emit(&mut writer, &Entry::Searches { fact: *fact, table })?;
        }
        if format.preserves_contributions() {
            for (&(element, property, target), contribution) in &overlay.reference_contributions {
                if dependency.is_some_and(|d| d.model().element(element).is_some()) {
                    continue;
                }
                let proof = tables.proof(contribution.explanation(), &mut writer)?;
                let searches = tables.search(contribution.searches(), &mut writer)?;
                emit(
                    &mut writer,
                    &Entry::ReferenceContribution {
                        element,
                        property,
                        target,
                        position: contribution.position(),
                        proof,
                        searches,
                    },
                )?;
            }
        }
    }
    emit(&mut writer, &Entry::End)
}

struct Restorer {
    proofs: Vec<Arc<Explanation>>,
    proof_pool: ExplanationPool,
    searches: Vec<Arc<BTreeSet<StructuralSearch>>>,
    search_pool: crate::derived::StructuralSearchPool,
}
impl Restorer {
    fn proof(&self, index: usize) -> Result<Arc<Explanation>, ArchiveError> {
        self.proofs
            .get(index)
            .cloned()
            .ok_or(ArchiveError::Invalid("proof table index"))
    }
    fn search(&self, index: usize) -> Result<Arc<BTreeSet<StructuralSearch>>, ArchiveError> {
        self.searches
            .get(index)
            .cloned()
            .ok_or(ArchiveError::Invalid("search table index"))
    }
    fn origin(&self, origin: StoredOrigin) -> Result<Origin, ArchiveError> {
        Ok(match origin {
            StoredOrigin::Declared(v) => Origin::Declared(v),
            StoredOrigin::Proof(i) => Origin::Derived(self.proof(i)?),
        })
    }
    fn slot(&self, slot: StoredSlot) -> Result<Slot, ArchiveError> {
        Ok(Slot {
            value: slot.value.restore()?,
            origin: self.origin(slot.origin)?,
        })
    }
    fn record(&self, record: Record) -> Result<ElementRecord, ArchiveError> {
        let mut slots = BTreeMap::new();
        for (id, slot) in record.slots {
            if slots.insert(id, self.slot(slot)?).is_some() {
                return Err(ArchiveError::Invalid("duplicate property"));
            }
        }
        Ok(ElementRecord {
            id: record.id,
            metaclass: record.class,
            origin: self.origin(record.origin)?,
            slots,
        })
    }
    fn occurrence(&self, occurrence: Occurrence) -> Result<AssociationOccurrence, ArchiveError> {
        let ends_len = occurrence.ends.len();
        let positions_len = occurrence.positions.len();
        let ends: BTreeMap<_, _> = occurrence.ends.into_iter().collect();
        let positions: BTreeMap<_, _> = occurrence.positions.into_iter().collect();
        if ends_len != ends.len() || positions_len != positions.len() {
            return Err(ArchiveError::Invalid("duplicate association end"));
        }
        Ok(AssociationOccurrence {
            id: occurrence.id,
            association: occurrence.association,
            ends,
            positions,
            origin: self.origin(occurrence.origin)?,
        })
    }
    fn failure(&self, failure: Failure) -> Result<ComputationFailure, ArchiveError> {
        Ok(match failure {
            Failure::Incomplete {
                reason,
                proof,
                searches,
            } => ComputationFailure::Incomplete {
                reason,
                explanation: (*self.proof(proof)?).clone(),
                searches: (*self.search(searches)?).clone(),
            },
            Failure::Invalid {
                diagnostic,
                proof,
                searches,
            } => ComputationFailure::Invalid {
                diagnostic,
                explanation: (*self.proof(proof)?).clone(),
                searches: (*self.search(searches)?).clone(),
            },
        })
    }
}
fn next(reader: &mut impl BufRead, line: &mut Vec<u8>) -> Result<Entry, ArchiveError> {
    line.clear();
    let count = std::io::Read::take(&mut *reader, MAX_LINE_BYTES + 1).read_until(b'\n', line)?;
    if count == 0 || count as u64 > MAX_LINE_BYTES {
        return Err(ArchiveError::Invalid("truncated or oversized entry"));
    }
    Ok(serde_json::from_slice(line)?)
}

/// Reconstruct one root declared snapshot under the exact supplied registry.
/// Retired identity reservations survive the round trip.
pub fn read_snapshot(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
) -> Result<Snapshot, ArchiveError> {
    let (snapshot, overlay) = read(&mut reader, registry, None)?;
    if overlay.is_some() {
        return Err(ArchiveError::Invalid("expected declared snapshot archive"));
    }
    Ok(snapshot)
}
/// Validate and reconstruct an overlay without executing language rules or issuing
/// publication acceptance. A language layer must authenticate its own trusted pin.
pub fn read_overlay(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
) -> Result<DerivedOverlay, ArchiveError> {
    let (_, overlay) = read(&mut reader, registry, None)?;
    overlay.ok_or(ArchiveError::Invalid("expected overlay archive"))
}
/// Restore a dependent graph delta while retaining the exact supplied dependency
/// allocation. The archive pins its bytes and cannot replace protected facts.
pub fn read_dependent_overlay(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Arc<DerivedOverlay>,
) -> Result<DerivedOverlay, ArchiveError> {
    let (_, overlay) = read(&mut reader, registry, Some(dependency))?;
    overlay.ok_or(ArchiveError::Invalid("expected dependent overlay archive"))
}

/// Validate a strict dependent evidence archive, preserving selected ordered
/// proofs/searches and the supplied immutable dependency allocation. This rejects
/// legacy and scheduler-frontier formats and confers no language acceptance.
pub fn read_dependent_overlay_with_evidence(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Arc<DerivedOverlay>,
) -> Result<DerivedOverlay, ArchiveError> {
    match read_input(
        &mut reader,
        registry,
        Some(dependency),
        false,
        ArchiveFormat::DependentEvidence,
    )?
    .1
    {
        Some(Frontier::Strict(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected strict dependent evidence")),
    }
}

/// Restore an unaccepted strict frontier under the exact supplied dependency.
pub fn read_publication_frontier(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
) -> Result<DerivedOverlay, ArchiveError> {
    match read_input(
        &mut reader,
        registry,
        dependency,
        false,
        ArchiveFormat::Frontier,
    )?
    .1
    {
        Some(Frontier::Strict(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected strict frontier")),
    }
}

/// Restore construction without promotion, rechecking present values, ownership,
/// proof dependencies and cycles while retaining lower-bound obligations.
pub fn read_construction_frontier(
    mut reader: impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
) -> Result<crate::derived::ConstructionOverlay, ArchiveError> {
    match read_input(
        &mut reader,
        registry,
        dependency,
        true,
        ArchiveFormat::Frontier,
    )?
    .1
    {
        Some(Frontier::Construction(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected construction frontier")),
    }
}

/// Restore a strict unaccepted frontier on the caller's exact declared snapshot.
/// The decoded declarations, provenance, reserved identities, registry and
/// immutable dependency must equal this input before it is adopted. A fresh
/// revision label is permitted, but no canonical assertion may change. Full
/// overlay validation then uses this original snapshot; no acceptance is issued.
pub fn read_publication_frontier_on(
    mut reader: impl BufRead,
    declared: Snapshot,
) -> Result<DerivedOverlay, ArchiveError> {
    match read_input_on(
        &mut reader,
        declared.model().registry.clone(),
        declared.immutable_dependency().cloned(),
        false,
        ArchiveFormat::Frontier,
        Some(DerivationInput::Strict(declared)),
    )?
    .1
    {
        Some(Frontier::Strict(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected strict frontier")),
    }
}

/// Restore a local frontier against exact source-derived declarations and the
/// caller's independently authenticated dependency identity. Protected facts and
/// reservations come only from that supplied dependency, which remains shared.
/// Exact declared assertions, registry, provenance, reservations and ordinary
/// overlay validation remain mandatory. The result is unaccepted cache data.
pub fn read_bound_frontier_on(
    mut reader: impl BufRead,
    declared: Snapshot,
    dependency_identity: [u8; 32],
) -> Result<DerivedOverlay, ArchiveError> {
    if declared.immutable_dependency().is_none() {
        return Err(ArchiveError::Invalid("missing immutable dependency"));
    }
    match read_input_on(
        &mut reader,
        declared.model().registry.clone(),
        declared.immutable_dependency().cloned(),
        false,
        ArchiveFormat::BoundFrontier(dependency_identity),
        Some(DerivationInput::Strict(declared)),
    )?
    .1
    {
        Some(Frontier::Strict(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected strict bound frontier")),
    }
}

/// Restore an unaccepted construction frontier on the original candidate Arc.
/// Exact declared input authentication includes lower-bound obligations and
/// retired identity reservations. Fresh revision labels are permitted; changed
/// declarations or evidence are rejected. The supplied Arc becomes the declared
/// input for every subsequent frontier and normal kernel validation is retained.
/// This never promotes a construction candidate or confers publication authority.
pub fn read_construction_frontier_on(
    mut reader: impl BufRead,
    declared: Arc<ConstructionView>,
) -> Result<crate::derived::ConstructionOverlay, ArchiveError> {
    match read_input_on(
        &mut reader,
        declared.model().registry.clone(),
        declared.immutable_dependency().cloned(),
        true,
        ArchiveFormat::Frontier,
        Some(DerivationInput::Construction(declared)),
    )?
    .1
    {
        Some(Frontier::Construction(overlay)) => Ok(overlay),
        _ => Err(ArchiveError::Invalid("expected construction frontier")),
    }
}

enum Frontier {
    Strict(DerivedOverlay),
    Construction(crate::derived::ConstructionOverlay),
}

#[allow(clippy::too_many_arguments)]
fn declared_input(
    registry: Arc<MetamodelRegistry>,
    revision: RevisionId,
    mut records: SharedMap<ElementId, Arc<ElementRecord>>,
    mut links: SharedMap<AssociationOccurrenceId, AssociationOccurrence>,
    used_ids: BTreeSet<ElementId>,
    used_links: BTreeSet<AssociationOccurrenceId>,
    dependency: Option<Arc<DerivedOverlay>>,
    construction: bool,
) -> Result<DerivationInput, ArchiveError> {
    if records.values().any(|r| {
        !matches!(r.origin, Origin::Declared(_))
            || r.slots
                .values()
                .any(|s| !matches!(s.origin, Origin::Declared(_)))
    }) || links
        .values()
        .any(|o| !matches!(o.origin, Origin::Declared(_)))
    {
        return Err(ArchiveError::Invalid("inferred fact in declared snapshot"));
    }
    if records.keys().any(|id| !used_ids.contains(id))
        || links.keys().any(|id| !used_links.contains(id))
    {
        return Err(ArchiveError::Invalid("missing reserved identity"));
    }
    for record in records.values() {
        for (property, _) in record.slots() {
            if registry
                .property(property)
                .map_err(ModelError::from)?
                .derived
            {
                return Err(ModelError::DerivedWrite {
                    element: record.id(),
                    property,
                }
                .into());
            }
        }
    }
    let navigation = if let Some(dependency) = &dependency {
        registry
            .require_extension_of(dependency.model().registry())
            .map_err(ModelError::from)?;
        let base = Snapshot::with_immutable_dependency(dependency.clone());
        if !base.inner.used_ids.iter().all(|id| used_ids.contains(id))
            || !base
                .inner
                .used_links
                .iter()
                .all(|id| used_links.contains(id))
        {
            return Err(ArchiveError::Invalid(
                "missing dependency identity reservation",
            ));
        }
        if records.keys().any(|id| base.has_used(*id))
            || links.keys().any(|id| base.has_used_occurrence(*id))
        {
            return Err(ArchiveError::Invalid("protected dependency identity"));
        }
        records = records.with_base(&dependency.model().records);
        links = links.with_base(&dependency.model().links);
        dependency.model().derived_navigation.fork()
    } else {
        SharedMap::new()
    };
    let mut validation = Validation {
        deficits: construction.then(BTreeMap::new),
    };
    let mut model = ModelView::build_with_validation(
        registry.clone(),
        records,
        links,
        navigation,
        &mut validation,
        dependency.as_ref().map(|d| d.model().indexes.clone()),
    )?;
    if let Some(dependency) = &dependency {
        model.declared_source = dependency.model().declared_source.clone();
        model.statuses = dependency.model().statuses.fork();
        model.searches = dependency.model().searches.fork();
        // Evidence on the separately supplied immutable dependency remains
        // authenticated by that exact object. Lossless archive formats restore
        // local selected contributions after constructing the overlay.
        model.reference_contributions = dependency.model().reference_contributions.fork();
    }
    let mut retained_ids = dependency
        .as_ref()
        .map_or_else(SharedSet::new, |d| d.element_reservations().fork());
    retained_ids.extend(used_ids);
    let mut retained_links = dependency
        .as_ref()
        .map_or_else(SharedSet::new, |d| d.occurrence_reservations().fork());
    retained_links.extend(used_links);
    if construction {
        let base = dependency.as_ref().map_or_else(
            || Snapshot::new(registry),
            |dependency| Snapshot::with_immutable_dependency(dependency.clone()),
        );
        base.check_dependency_ownership(&model)?;
        return Ok(DerivationInput::Construction(Arc::new(ConstructionView {
            base,
            used_ids: retained_ids,
            used_links: retained_links,
            revision,
            model,
            dependency,
            obligations: validation
                .deficits
                .expect("construction validation")
                .into_values()
                .collect(),
        })));
    }
    let snapshot = Snapshot {
        inner: Arc::new(SnapshotData {
            revision,
            model,
            used_ids: retained_ids,
            used_links: retained_links,
            dependency,
        }),
    };
    snapshot.check_dependency_ownership(snapshot.model())?;
    Ok(DerivationInput::Strict(snapshot))
}
fn read(
    reader: &mut impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
) -> Result<(Snapshot, Option<DerivedOverlay>), ArchiveError> {
    let (input, overlay) = read_input(reader, registry, dependency, false, ArchiveFormat::Legacy)?;
    let DerivationInput::Strict(snapshot) = input else {
        unreachable!("strict read")
    };
    let overlay = overlay.map(|overlay| match overlay {
        Frontier::Strict(overlay) => overlay,
        Frontier::Construction(_) => unreachable!("strict read"),
    });
    Ok((snapshot, overlay))
}

fn read_input(
    reader: &mut impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
    construction: bool,
    archive_format: ArchiveFormat,
) -> Result<(DerivationInput, Option<Frontier>), ArchiveError> {
    read_input_on(
        reader,
        registry,
        dependency,
        construction,
        archive_format,
        None,
    )
}

fn read_input_on(
    reader: &mut impl BufRead,
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
    construction: bool,
    archive_format: ArchiveFormat,
    mut supplied_input: Option<DerivationInput>,
) -> Result<(DerivationInput, Option<Frontier>), ArchiveError> {
    let mut line = Vec::new();
    let has_overlay = match (next(reader, &mut line)?, &dependency) {
        (
            Entry::Header {
                format,
                registry: expected,
                overlay,
            },
            None,
        ) if format == archive_format.identity(construction, false)
            && expected == registry_digest(&registry) =>
        {
            overlay
        }
        (
            Entry::DependentHeader {
                format,
                registry: expected,
                dependency: digest,
            },
            Some(dependency),
        ) if format == archive_format.identity(construction, true)
            && expected == registry_digest(&registry)
            && digest == dependency_digest(dependency, archive_format)? =>
        {
            true
        }
        _ => {
            return Err(ArchiveError::Invalid(
                "format, registry or dependency mismatch",
            ));
        }
    };
    let Entry::Snapshot {
        revision,
        mut used_ids,
        mut used_links,
    } = next(reader, &mut line)?
    else {
        return Err(ArchiveError::Invalid("snapshot header"));
    };
    if matches!(archive_format, ArchiveFormat::BoundFrontier(_)) {
        let dependency = dependency
            .as_ref()
            .ok_or(ArchiveError::Invalid("missing immutable dependency"))?;
        if used_ids
            .iter()
            .any(|id| dependency.element_reservations().contains(id))
            || used_links
                .iter()
                .any(|id| dependency.occurrence_reservations().contains(id))
        {
            return Err(ArchiveError::Invalid(
                "protected reservation in local frontier",
            ));
        }
        used_ids.extend(dependency.element_reservations().iter().copied());
        used_links.extend(dependency.occurrence_reservations().iter().copied());
    }
    let mut restorer = Restorer {
        proofs: vec![],
        proof_pool: ExplanationPool::default(),
        searches: vec![],
        search_pool: crate::derived::StructuralSearchPool::default(),
    };
    let mut records = SharedMap::new();
    let mut links = SharedMap::new();
    let mut navigation = SharedMap::new();
    let mut failures = SharedMap::new();
    let mut searches = SharedMap::new();
    let mut contributions = SharedMap::new();
    let mut snapshot = None;
    let mut changed_records = BTreeSet::new();
    let mut changed_links = BTreeSet::new();
    loop {
        match next(reader, &mut line)? {
            Entry::Proof(proof) => restorer.proofs.push(restorer.proof_pool.intern(proof)),
            Entry::Search(search) => restorer
                .searches
                .push(restorer.search_pool.intern_shared(Arc::new(search))),
            Entry::Record(record) => {
                if dependency.as_ref().is_some_and(|d| {
                    d.model().element(record.id).is_some() || d.declared().has_used(record.id)
                }) {
                    return Err(ArchiveError::Invalid("protected dependency record"));
                }
                if !changed_records.insert(record.id) {
                    return Err(ArchiveError::Invalid("duplicate record"));
                }
                let record = restorer.record(record)?;
                records.insert(record.id, Arc::new(record));
            }
            Entry::Occurrence(occurrence) => {
                if dependency.as_ref().is_some_and(|d| {
                    d.model().association_occurrence(occurrence.id).is_some()
                        || d.declared().has_used_occurrence(occurrence.id)
                }) {
                    return Err(ArchiveError::Invalid("protected dependency occurrence"));
                }
                if !changed_links.insert(occurrence.id) {
                    return Err(ArchiveError::Invalid("duplicate occurrence"));
                }
                let occurrence = restorer.occurrence(occurrence)?;
                links.insert(occurrence.id, occurrence);
            }
            Entry::Overlay if has_overlay && snapshot.is_none() => {
                let declared = declared_input(
                    registry.clone(),
                    revision,
                    std::mem::take(&mut records),
                    std::mem::take(&mut links),
                    used_ids.clone(),
                    used_links.clone(),
                    dependency.clone(),
                    construction,
                )?;
                let declared = if let Some(supplied) = supplied_input.take() {
                    if !declared.matches_archive_input(&supplied) {
                        return Err(ArchiveError::Invalid("frontier declared input mismatch"));
                    }
                    supplied
                } else {
                    declared
                };
                records = declared.model().records.clone();
                links = declared.model().links.clone();
                snapshot = Some(declared);
                changed_records.clear();
                changed_links.clear();
            }
            Entry::Navigation {
                element,
                property,
                slot,
            } if snapshot.is_some() => {
                if dependency
                    .as_ref()
                    .is_some_and(|d| d.model().element(element).is_some())
                {
                    return Err(ArchiveError::Invalid("protected dependency navigation"));
                }
                if navigation
                    .insert((element, property), restorer.slot(slot)?)
                    .is_some()
                {
                    return Err(ArchiveError::Invalid("duplicate navigation"));
                }
            }
            Entry::Failure {
                element,
                property,
                failure,
            } if snapshot.is_some() => {
                if dependency
                    .as_ref()
                    .is_some_and(|d| d.model().element(element).is_some())
                {
                    return Err(ArchiveError::Invalid("protected dependency status"));
                }
                if failures
                    .insert((element, property), restorer.failure(failure)?)
                    .is_some()
                {
                    return Err(ArchiveError::Invalid("duplicate failure"));
                }
            }
            Entry::Searches { fact, table } if snapshot.is_some() => {
                if dependency.as_ref().is_some_and(|d| match fact {
                    FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                        d.model().element(id).is_some()
                    }
                    FactKey::AssociationOccurrence(id) => {
                        d.model().association_occurrence(id).is_some()
                    }
                }) {
                    return Err(ArchiveError::Invalid("protected dependency search"));
                }
                if searches.insert(fact, restorer.search(table)?).is_some() {
                    return Err(ArchiveError::Invalid("duplicate computation search"));
                }
            }
            Entry::ReferenceContribution {
                element,
                property,
                target,
                position,
                proof,
                searches,
            } if archive_format.preserves_contributions() && snapshot.is_some() => {
                if dependency
                    .as_ref()
                    .is_some_and(|d| d.model().element(element).is_some())
                {
                    return Err(ArchiveError::Invalid("protected dependency contribution"));
                }
                let contribution = Arc::new(crate::derived::OrderedReferenceContribution {
                    position,
                    explanation: restorer.proof(proof)?,
                    searches: restorer.search(searches)?,
                });
                if contributions
                    .insert((element, property, target), contribution)
                    .is_some()
                {
                    return Err(ArchiveError::Invalid("duplicate ordered contribution"));
                }
            }
            Entry::End => break,
            _ => return Err(ArchiveError::Invalid("unexpected entry")),
        }
    }
    if !reader.fill_buf()?.is_empty() {
        return Err(ArchiveError::Invalid("trailing content"));
    }
    // Canonical records now retain every referenced proof/search allocation.
    // Release decoding tables and their cached proof adjacency before rebuilding
    // indexes and validating the complete graph, avoiding two retained pools.
    drop(restorer);
    drop(line);
    if let Some(snapshot) = snapshot {
        if let Some(dependency) = &dependency {
            navigation = navigation.with_base(&dependency.model().derived_navigation);
            failures = failures.with_base(&dependency.model().statuses);
            searches = searches.with_base(&dependency.model().searches);
        }
        let (mut model, obligations) =
            snapshot.build_model(registry, records, links, navigation)?;
        model.statuses = failures;
        model.searches = searches;
        for (&(element, property, target), contribution) in &contributions {
            if !model.navigation_slot(element, property).is_some_and(|slot| matches!(slot.value(), SlotValue::Ordered(values) if values.get(contribution.position()) == Some(&Value::Reference(target)))) {
                return Err(ArchiveError::Invalid("ordered contribution target/position"));
            }
        }
        model.reference_contributions = contributions;
        if let Some(dependency) = &dependency {
            model.reference_contributions = model
                .reference_contributions
                .with_base(&dependency.model().reference_contributions);
        }
        let overlay = match &snapshot {
            DerivationInput::Strict(declared) => {
                Frontier::Strict(DerivedOverlay::restore_archive(declared.clone(), model)?)
            }
            DerivationInput::Construction(declared) => {
                Frontier::Construction(crate::derived::ConstructionOverlay::restore_archive(
                    declared.clone(),
                    model,
                    obligations,
                )?)
            }
        };
        Ok((snapshot, Some(overlay)))
    } else if has_overlay {
        Err(ArchiveError::Invalid("missing overlay"))
    } else {
        Ok((
            declared_input(
                registry,
                revision,
                records,
                links,
                used_ids,
                used_links,
                dependency,
                construction,
            )?,
            None,
        ))
    }
}
