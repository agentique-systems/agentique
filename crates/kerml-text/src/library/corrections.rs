//! Reviewed changes to canonical model facts, never to source text or syntax.
use super::*;
use agq_kernel::{
    provenance::{DeclaredOrigin, Origin},
    value::{SlotValue, Value},
};
use serde::Deserialize;
use std::{collections::BTreeSet, sync::Arc};

/// A source-qualified canonical selector. Display names never select patch inputs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedElementSelector {
    pub id: String,
    pub document: String,
    pub sha256: String,
    pub range: [u64; 2],
    pub metaclass: String,
}

/// Identity of a pinned fact or a separately allocated correction output.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchElement {
    Pinned(String),
    Output(String),
}

/// Only the literal and reference forms needed by the reviewed correction set.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchValue {
    Boolean(bool),
    Integer(String),
    String(String),
    Enumeration(String),
    Reference(PatchElement),
}

/// Explicit canonical changes. Each change has a separate provenance output key.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum LibraryPatchOperation {
    Create {
        key: String,
        metaclass: String,
    },
    Reclassify {
        key: String,
        element: PatchElement,
        metaclass: String,
    },
    Set {
        key: String,
        element: PatchElement,
        property: String,
        value: PatchValue,
    },
    Append {
        key: String,
        element: PatchElement,
        property: String,
        target: PatchElement,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryCorrectionEntry {
    pub id: String,
    pub document: String,
    pub authority: BTreeSet<String>,
    pub operations: Vec<LibraryPatchOperation>,
}

/// Reviewed model correction data, distinct from descriptor and algorithm errata.
#[derive(Clone, Debug, Deserialize)]
pub struct OperationalLibraryPatchSet {
    pub profile_id: String,
    pub library_set: String,
    pub selectors: BTreeMap<String, PinnedElementSelector>,
    pub entries: Vec<LibraryCorrectionEntry>,
}

fn failure(message: impl Into<String>) -> LibraryLoadError {
    LibraryLoadError::Interpretation(message.into())
}
fn uuid(value: &str) -> Result<u128, LibraryLoadError> {
    uuid::Uuid::parse_str(value)
        .map(|id| id.as_u128())
        .map_err(|_| failure("Invalid reviewed semantic identity"))
}

impl OperationalLibraryPatchSet {
    /// Load the frozen implemented review, rejecting changed manifest bytes.
    pub fn reviewed() -> Result<Self, LibraryLoadError> {
        agq_kerml::descriptors_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V3)
            .map_err(|e| failure(e.to_string()))?;
        serde_json::from_str(agq_kerml::OPERATIONAL_LIBRARY_ERRATA_V3_MANIFEST)
            .map_err(|e| failure(e.to_string()))
    }
    /// Private, versioned and unambiguously encoded identity domain. Reference-XMI
    /// identifiers are never imported as Agentique canonical identifiers.
    pub fn output_id(&self, library: agq_kernel::LibraryId, entry: &str, key: &str) -> ElementId {
        let encoded = serde_json::to_vec(&(
            "agentique-operational-library-output/1",
            &self.profile_id,
            &self.library_set,
            library.to_string(),
            entry,
            key,
        ))
        .expect("identity tuple");
        ElementId::from_u128(
            uuid::Uuid::new_v5(
                &uuid::Uuid::from_u128(0x802a939427b154e9b496e1d88bd98722),
                &encoded,
            )
            .as_u128(),
        )
    }

    /// Apply to a separate canonical construction. All preconditions are checked
    /// against the original input before any corrected model can be returned.
    pub(super) fn apply(
        &self,
        draft: LibraryDraft,
        sources: &VerifiedLibrarySet,
    ) -> Result<LibraryDraft, LibraryLoadError> {
        if !draft.profile.accepts_correction_profile(&self.profile_id)
            || sources.content_set_id() != self.library_set
        {
            return Err(failure(
                "Correction profile or exact library content set mismatch",
            ));
        }
        let model = draft.candidate.model();
        let registry = model.registry();
        let mut selected = BTreeMap::new();
        for (key, selector) in &self.selectors {
            let id = ElementId::from_u128(uuid(&selector.id)?);
            let record = model
                .element(id)
                .ok_or_else(|| failure(format!("Missing patch input {key}")))?;
            let source = draft
                .source_map
                .get(&FactKey::Element(id))
                .ok_or_else(|| failure("Patch input lacks pinned source"))?;
            let document = sources
                .documents()
                .find(|d| d.document() == source.document)
                .ok_or_else(|| failure("Patch source document missing"))?;
            if document.path() != selector.document
                || document.sha256() != selector.sha256
                || [source.range.start(), source.range.end()] != selector.range
                || record.metaclass() != MetaclassId::from_u128(uuid(&selector.metaclass)?)
                || record.origin() != &Origin::Declared(document.origin())
            {
                return Err(failure(format!("Reviewed selector mismatch: {key}")));
            }
            selected.insert(key.clone(), id);
        }
        let mut outputs = BTreeMap::new();
        let mut classes = BTreeMap::new();
        let mut element_origins = BTreeMap::new();
        let mut slots = BTreeMap::<(ElementId, PropertyId), (SlotValue, DeclaredOrigin)>::new();
        let mut written = BTreeSet::new();
        for entry in &self.entries {
            let document = sources
                .documents()
                .find(|d| d.path() == entry.document)
                .ok_or_else(|| failure("Correction document missing"))?;
            if entry.authority.is_empty() {
                return Err(failure("Correction authority missing"));
            }
            for operation in &entry.operations {
                if let LibraryPatchOperation::Create { key, metaclass } = operation {
                    let id = self.output_id(document.library(), &entry.id, key);
                    if model.element(id).is_some()
                        || outputs.insert(format!("{}/{key}", entry.id), id).is_some()
                    {
                        return Err(failure("Correction output identity collision"));
                    }
                    classes.insert(id, MetaclassId::from_u128(uuid(metaclass)?));
                }
            }
        }
        for entry in &self.entries {
            let document = sources
                .documents()
                .find(|d| d.path() == entry.document)
                .expect("checked input");
            let resolve = |reference: &PatchElement| -> Result<ElementId, LibraryLoadError> {
                match reference {
                    PatchElement::Pinned(key) => selected.get(key),
                    PatchElement::Output(key) => outputs.get(&format!("{}/{key}", entry.id)),
                }
                .copied()
                .ok_or_else(|| failure("Unknown correction reference"))
            };
            for operation in &entry.operations {
                let key = match operation {
                    LibraryPatchOperation::Create { key, .. }
                    | LibraryPatchOperation::Reclassify { key, .. }
                    | LibraryPatchOperation::Set { key, .. }
                    | LibraryPatchOperation::Append { key, .. } => key,
                };
                if !written.insert((&entry.id, key)) {
                    return Err(failure("Duplicate correction operation key"));
                }
                let origin = DeclaredOrigin::ReviewedCorrection {
                    profile: self.profile_id.clone(),
                    entry: entry.id.clone(),
                    authority: entry.authority.clone(),
                    library: document.library(),
                    source_key: format!("{}#sha256:{}", document.path(), document.sha256()),
                    output_key: key.clone(),
                };
                match operation {
                    LibraryPatchOperation::Create { key, .. } => {
                        let id = outputs[&format!("{}/{key}", entry.id)];
                        element_origins.insert(id, origin.clone());
                        slots.insert(
                            (id, agq_kerml::properties::ELEMENT_ELEMENT_ID),
                            (SlotValue::Scalar(Value::String(id.to_string())), origin),
                        );
                    }
                    LibraryPatchOperation::Reclassify {
                        element, metaclass, ..
                    } => {
                        let id = resolve(element)?;
                        classes.insert(id, MetaclassId::from_u128(uuid(metaclass)?));
                        element_origins.insert(id, origin);
                    }
                    LibraryPatchOperation::Set {
                        element,
                        property,
                        value,
                        ..
                    } => {
                        let id = resolve(element)?;
                        let property = PropertyId::from_u128(uuid(property)?);
                        let value = match value {
                            PatchValue::Boolean(value) => Value::Boolean(*value),
                            PatchValue::Integer(value) => Value::Integer(
                                value
                                    .parse()
                                    .map_err(|_| failure("Invalid correction Integer"))?,
                            ),
                            PatchValue::String(value) => Value::String(value.clone()),
                            PatchValue::Reference(target) => Value::Reference(resolve(target)?),
                            PatchValue::Enumeration(value) => Value::Enumeration(
                                agq_kernel::EnumerationLiteralId::from_u128(uuid(value)?),
                            ),
                        };
                        if slots
                            .insert((id, property), (SlotValue::Scalar(value), origin))
                            .is_some()
                        {
                            return Err(failure("Repeated correction slot replacement"));
                        }
                    }
                    LibraryPatchOperation::Append {
                        element,
                        property,
                        target,
                        ..
                    } => {
                        let id = resolve(element)?;
                        let property = PropertyId::from_u128(uuid(property)?);
                        let base = slots
                            .get(&(id, property))
                            .map(|s| s.0.clone())
                            .or_else(|| {
                                model
                                    .navigation_slot(id, property)
                                    .map(|s| s.value().clone())
                            })
                            .unwrap_or_else(|| SlotValue::Ordered(vec![]));
                        let SlotValue::Ordered(mut values) = base else {
                            return Err(failure("Append requires ordered canonical storage"));
                        };
                        values.push(Value::Reference(resolve(target)?));
                        slots.insert((id, property), (SlotValue::Ordered(values), origin));
                    }
                }
            }
        }
        let empty = Snapshot::new(Arc::new(registry.clone()));
        let mut changes = empty.change_set();
        for record in model.elements() {
            let Origin::Declared(origin) = record.origin() else {
                return Err(failure("Correction input must be declared canonical facts"));
            };
            changes.create(
                record.id(),
                classes
                    .get(&record.id())
                    .copied()
                    .unwrap_or(record.metaclass()),
                element_origins.get(&record.id()).unwrap_or(origin).clone(),
            );
            for (property, slot) in record.slots() {
                if !slots.contains_key(&(record.id(), property)) {
                    let Origin::Declared(origin) = slot.origin() else {
                        return Err(failure("Correction input slot must be declared"));
                    };
                    changes.set(record.id(), property, slot.value().clone(), origin.clone());
                }
            }
        }
        for &id in outputs.values() {
            changes.create(id, classes[&id], element_origins[&id].clone());
        }
        for ((id, property), (value, origin)) in slots {
            changes.set(id, property, value, origin);
        }
        for link in model.association_occurrences() {
            changes.link(
                link.id(),
                link.association(),
                link.ends().clone(),
                link.positions().clone(),
                link.declared_origin()
                    .expect("declared library construction")
                    .clone(),
            );
        }
        let candidate = empty.preview(&changes)?;
        let (superseded_references, references) =
            draft.references.into_iter().partition(|reference| {
                candidate
                    .model()
                    .navigation_slot(reference.relationship, reference.property)
                    .is_some_and(|slot| {
                        matches!(
                            slot.origin(),
                            Origin::Declared(DeclaredOrigin::ReviewedCorrection { .. })
                        )
                    })
            });
        Ok(LibraryDraft {
            candidate,
            references,
            superseded_references,
            ..draft
        })
    }
}
