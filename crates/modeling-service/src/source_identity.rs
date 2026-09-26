//! Shared checks for service-proven, identity-preserving source commands.
//! Proofs and identity arenas remain internal; callers supply only edit intent.
use super::*;
use agq_kerml::properties;
use agq_kerml_semantics::Completeness;
use agq_kerml_syntax::{TextEdit, production};
use agq_kernel::{
    ElementId, PropertyId,
    provenance::{ByteRange, DeclaredOrigin, Origin},
    value::{SlotValue, Value},
};

fn invalid(reason: &str) -> ServiceError {
    ServiceError::Invalid(format!("Source identity continuity: {reason}"))
}

/// Restore only an internally proven arena over the authenticated predecessor.
pub(super) fn reconstruct(
    before: &WorkingProjectRevision,
    parsed: &production::Document,
    identities: Vec<production::NodeIdentity>,
    control: &CompilationControl,
) -> Result<(Arc<WorkingProjectRevision>, ProjectRevisionCheckpoint), ServiceError> {
    control.check()?;
    let mut checkpoint = before.checkpoint();
    checkpoint.project_revision_id = ProjectRevisionId::new();
    checkpoint.parent_revision_id = Some(before.revision());
    let saved = checkpoint
        .source
        .documents
        .iter_mut()
        .find(|saved| saved.document_id == parsed.document())
        .ok_or_else(|| invalid("authored source checkpoint is missing"))?;
    saved.source_revision_id = parsed.revision();
    saved.content_digest = ContentDigest::of(parsed.source().as_bytes()).0;
    saved.syntax_nodes = identities;
    let mut sources: BTreeMap<_, _> = before
        .documents()
        .map(|(_, document)| (document.id(), document.source().to_owned()))
        .collect();
    sources.insert(parsed.document(), parsed.source().into());
    let working = checkpoint
        .restore_sharing_dependency_controlled(before, &sources, control)
        .map_err(|error| {
            if error.is_cancelled() {
                ServiceError::Cancelled(agq_kerml_semantics::Cancelled)
            } else {
                invalid(&format!("ordinary reconstruction failed: {error}"))
            }
        })?;
    control.check()?;
    Ok((working, checkpoint))
}

pub(super) fn mapped_range(range: ByteRange, edit: &TextEdit) -> Result<ByteRange, ServiceError> {
    let delta = edit.replacement.len() as i64 - (edit.range.end() - edit.range.start()) as i64;
    if range.end() <= edit.range.start() {
        Ok(range)
    } else if range.start() >= edit.range.end() {
        ByteRange::new(
            (range.start() as i64 + delta) as u64,
            (range.end() as i64 + delta) as u64,
        )
        .map_err(|_| invalid("shifted node range is invalid"))
    } else if range.start() <= edit.range.start() && range.end() >= edit.range.end() {
        ByteRange::new(range.start(), (range.end() as i64 + delta) as u64)
            .map_err(|_| invalid("containing node range is invalid"))
    } else {
        Err(invalid("edit partially replaces an existing production"))
    }
}

/// Exact exceptions to declared-slot equality, never supplied by callers.
pub(super) enum Mutation<'a> {
    AppendMember {
        owner: ElementId,
    },
    Rename {
        element: ElementId,
        before: &'a str,
        after: &'a str,
    },
}
impl Mutation<'_> {
    fn permits(&self, element: ElementId, property: PropertyId) -> bool {
        match self {
            Self::AppendMember { owner } => {
                element == *owner && property == properties::ELEMENT_OWNED_RELATIONSHIP
            }
            Self::Rename {
                element: target, ..
            } => element == *target && property == properties::ELEMENT_DECLARED_NAME,
        }
    }
}

pub(super) fn verify_existing(
    before: &WorkingProjectRevision,
    after: &WorkingProjectRevision,
    mutation: Mutation<'_>,
) -> Result<(), ServiceError> {
    let old = before
        .strict_snapshot()
        .ok_or_else(|| invalid("predecessor declared graph unavailable"))?
        .model();
    let next = after
        .strict_snapshot()
        .ok_or_else(|| invalid("candidate declared graph unavailable"))?
        .model();
    let old_queries = before
        .kerml_queries()
        .map_err(|error| invalid(&format!("predecessor query: {error:?}")))?;
    let queries = after
        .kerml_queries()
        .map_err(|error| invalid(&format!("candidate query: {error:?}")))?;
    for record in old.elements().filter(|record| {
        matches!(
            record.origin(),
            Origin::Declared(DeclaredOrigin::Authored { source: Some(_) })
        )
    }) {
        let replacement = next
            .element(record.id())
            .ok_or_else(|| invalid("an existing declared identity disappeared"))?;
        if replacement.metaclass() != record.metaclass() {
            return Err(invalid("an existing declared kind changed"));
        }
        let before_owner = old_queries.owner(record.id());
        let after_owner = queries.owner(record.id());
        if before_owner.completeness != Completeness::Complete
            || after_owner.completeness != Completeness::Complete
            || before_owner.value != after_owner.value
        {
            return Err(invalid(
                "existing declared ownership changed or became incomplete",
            ));
        }
        let before_slots: BTreeMap<_, _> = record
            .slots()
            .filter(|(property, _)| !mutation.permits(record.id(), *property))
            .map(|(p, slot)| (p, slot.value()))
            .collect();
        let after_slots: BTreeMap<_, _> = replacement
            .slots()
            .filter(|(property, _)| !mutation.permits(record.id(), *property))
            .map(|(p, slot)| (p, slot.value()))
            .collect();
        if before_slots != after_slots {
            return Err(invalid(
                "an existing declaration changed outside the proven command",
            ));
        }
        match &mutation {
            Mutation::AppendMember { owner } if record.id() == *owner => {
                let previous: Vec<_> = record
                    .slot(properties::ELEMENT_OWNED_RELATIONSHIP)
                    .into_iter()
                    .flat_map(|slot| slot.value().values())
                    .collect();
                let current: Vec<_> = replacement
                    .slot(properties::ELEMENT_OWNED_RELATIONSHIP)
                    .into_iter()
                    .flat_map(|slot| slot.value().values())
                    .collect();
                if !current.starts_with(&previous) {
                    return Err(invalid("existing owner member order changed"));
                }
            }
            Mutation::Rename {
                element,
                before,
                after,
            } if record.id() == *element => {
                if record
                    .slot(properties::ELEMENT_DECLARED_NAME)
                    .map(|slot| slot.value())
                    != Some(&SlotValue::Scalar(Value::String((*before).into())))
                    || replacement
                        .slot(properties::ELEMENT_DECLARED_NAME)
                        .map(|slot| slot.value())
                        != Some(&SlotValue::Scalar(Value::String((*after).into())))
                {
                    return Err(invalid(
                        "the selected canonical declared name does not match the proven token edit",
                    ));
                }
            }
            _ => {}
        }
    }
    let references: BTreeMap<_, _> = after
        .references()
        .iter()
        .map(|reference| (reference.relationship, reference))
        .collect();
    for reference in before.references() {
        let next = references
            .get(&reference.relationship)
            .ok_or_else(|| invalid("an existing reference disappeared"))?;
        if reference.name != next.name
            || reference.specific != next.specific
            || reference.resolution.value != next.resolution.value
            || reference.resolution.completeness != next.resolution.completeness
            || next.resolution.completeness != Completeness::Complete
        {
            return Err(invalid(
                "an existing reference changed target; the name may break or shadow a binding",
            ));
        }
    }
    if matches!(mutation, Mutation::Rename { .. })
        && (before.references().len() != after.references().len()
            || old
                .elements()
                .map(|record| record.id())
                .collect::<std::collections::BTreeSet<_>>()
                != next
                    .elements()
                    .map(|record| record.id())
                    .collect::<std::collections::BTreeSet<_>>()
            || old
                .association_occurrences()
                .map(|record| record.id())
                .collect::<std::collections::BTreeSet<_>>()
                != next
                    .association_occurrences()
                    .map(|record| record.id())
                    .collect::<std::collections::BTreeSet<_>>())
    {
        return Err(invalid(
            "rename added or removed canonical records or references",
        ));
    }
    let before_checkpoint = before.checkpoint();
    let after_checkpoint = after.checkpoint();
    let retired = &before_checkpoint.source.identity_history.retired;
    let next_retired = &after_checkpoint.source.identity_history.retired;
    if !retired.elements.is_subset(&next_retired.elements)
        || !retired.occurrences.is_subset(&next_retired.occurrences)
    {
        return Err(invalid("retired identities were lost"));
    }
    if matches!(mutation, Mutation::Rename { .. })
        && (retired.elements != next_retired.elements
            || retired.occurrences != next_retired.occurrences)
    {
        return Err(invalid("rename changed retired identity reservations"));
    }
    if next
        .elements()
        .any(|record| retired.elements.contains(&record.id()))
        || next
            .association_occurrences()
            .any(|record| retired.occurrences.contains(&record.id()))
    {
        return Err(invalid("a retired canonical identity was resurrected"));
    }
    Ok(())
}
