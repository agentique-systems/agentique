//! Streaming, versioned kernel graph archives. Loading proves structural integrity,
//! never language publication, rule truth, or authority acceptance.
//!
//! Archives contain one root declared snapshot and optionally its derived overlay.
//! Protected dependency histories are intentionally rejected: flattening one would
//! erase its immutable boundary. Compression and trusted content pins belong to the
//! caller. A caller can wrap any `Read`/`Write` in its own compression stream.
use super::*;
use crate::derived::{ComputationFailure, DerivedOverlay, IncompleteReason, StructuralSearch};
use crate::provenance::{Explanation, ExplanationPool};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, Write};

const FORMAT: &str = "agq-kernel-graph-archive/1";
const MAX_LINE_BYTES: u64 = 64 * 1024 * 1024;

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
    write(snapshot, None, writer)
}
/// Stream a declared root snapshot and all derived state, with shared evidence tables.
pub fn write_overlay(overlay: &DerivedOverlay, writer: impl Write) -> Result<(), ArchiveError> {
    write(overlay.declared(), Some(overlay), writer)
}
fn write(
    snapshot: &Snapshot,
    overlay: Option<&DerivedOverlay>,
    mut writer: impl Write,
) -> Result<(), ArchiveError> {
    if snapshot.immutable_dependency().is_some() || snapshot.model().declared_source.is_some() {
        return Err(ArchiveError::Invalid(
            "protected dependency snapshots require their own archive boundary",
        ));
    }
    emit(
        &mut writer,
        &Entry::Header {
            format: FORMAT.into(),
            registry: registry_digest(snapshot.model().registry()),
            overlay: overlay.is_some(),
        },
    )?;
    emit(
        &mut writer,
        &Entry::Snapshot {
            revision: snapshot.revision(),
            used_ids: snapshot.inner.used_ids.clone(),
            used_links: snapshot.inner.used_links.clone(),
        },
    )?;
    let mut tables = Tables::new();
    for record in snapshot.model().elements() {
        tables.record(record, &mut writer)?;
    }
    for occurrence in snapshot.model().association_occurrences() {
        tables.occurrence(occurrence, &mut writer)?;
    }
    if let Some(overlay) = overlay {
        emit(&mut writer, &Entry::Overlay)?;
        for record in overlay.model().elements() {
            if snapshot.model().element(record.id()) != Some(record) {
                tables.record(record, &mut writer)?;
            }
        }
        for occurrence in overlay.model().association_occurrences() {
            if snapshot.model().association_occurrence(occurrence.id()) != Some(occurrence) {
                tables.occurrence(occurrence, &mut writer)?;
            }
        }
        for ((element, property), slot) in overlay.model().derived_navigation_results() {
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
        for (&(element, property), failure) in &overlay.model().statuses {
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
        for (fact, searches) in overlay.model().computation_searches() {
            let table = tables.search(searches, &mut writer)?;
            emit(&mut writer, &Entry::Searches { fact: *fact, table })?;
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
    let (snapshot, overlay) = read(&mut reader, registry)?;
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
    let (_, overlay) = read(&mut reader, registry)?;
    overlay.ok_or(ArchiveError::Invalid("expected overlay archive"))
}
fn declared_snapshot(
    registry: Arc<MetamodelRegistry>,
    revision: RevisionId,
    records: BTreeMap<ElementId, Arc<ElementRecord>>,
    links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
    used_ids: BTreeSet<ElementId>,
    used_links: BTreeSet<AssociationOccurrenceId>,
) -> Result<Snapshot, ArchiveError> {
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
    let model = ModelView::build(registry, records, links, BTreeMap::new())?;
    Ok(Snapshot {
        inner: Arc::new(SnapshotData {
            revision,
            model,
            used_ids,
            used_links,
            dependency: None,
        }),
    })
}
fn read(
    reader: &mut impl BufRead,
    registry: Arc<MetamodelRegistry>,
) -> Result<(Snapshot, Option<DerivedOverlay>), ArchiveError> {
    let mut line = Vec::new();
    let Entry::Header {
        format,
        registry: expected,
        overlay: has_overlay,
    } = next(reader, &mut line)?
    else {
        return Err(ArchiveError::Invalid("header"));
    };
    if format != FORMAT || expected != registry_digest(&registry) {
        return Err(ArchiveError::Invalid("format or exact registry mismatch"));
    }
    let Entry::Snapshot {
        revision,
        used_ids,
        used_links,
    } = next(reader, &mut line)?
    else {
        return Err(ArchiveError::Invalid("snapshot header"));
    };
    let mut restorer = Restorer {
        proofs: vec![],
        proof_pool: ExplanationPool::default(),
        searches: vec![],
        search_pool: crate::derived::StructuralSearchPool::default(),
    };
    let mut records = BTreeMap::new();
    let mut links = BTreeMap::new();
    let mut navigation = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut searches = BTreeMap::new();
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
                if !changed_records.insert(record.id) {
                    return Err(ArchiveError::Invalid("duplicate record"));
                }
                let record = restorer.record(record)?;
                records.insert(record.id, Arc::new(record));
            }
            Entry::Occurrence(occurrence) => {
                if !changed_links.insert(occurrence.id) {
                    return Err(ArchiveError::Invalid("duplicate occurrence"));
                }
                let occurrence = restorer.occurrence(occurrence)?;
                links.insert(occurrence.id, occurrence);
            }
            Entry::Overlay if has_overlay && snapshot.is_none() => {
                let declared = declared_snapshot(
                    registry.clone(),
                    revision,
                    std::mem::take(&mut records),
                    std::mem::take(&mut links),
                    used_ids.clone(),
                    used_links.clone(),
                )?;
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
                if failures
                    .insert((element, property), restorer.failure(failure)?)
                    .is_some()
                {
                    return Err(ArchiveError::Invalid("duplicate failure"));
                }
            }
            Entry::Searches { fact, table } if snapshot.is_some() => {
                if searches.insert(fact, restorer.search(table)?).is_some() {
                    return Err(ArchiveError::Invalid("duplicate computation search"));
                }
            }
            Entry::End => break,
            _ => return Err(ArchiveError::Invalid("unexpected entry")),
        }
    }
    if !reader.fill_buf()?.is_empty() {
        return Err(ArchiveError::Invalid("trailing content"));
    }
    if let Some(snapshot) = snapshot {
        let mut model = ModelView::build(registry, records, links, navigation)?;
        model.statuses = failures;
        model.searches = searches;
        let overlay = DerivedOverlay::restore_archive(snapshot.clone(), model)?;
        Ok((snapshot, Some(overlay)))
    } else if has_overlay {
        Err(ArchiveError::Invalid("missing overlay"))
    } else {
        Ok((
            declared_snapshot(registry, revision, records, links, used_ids, used_links)?,
            None,
        ))
    }
}
