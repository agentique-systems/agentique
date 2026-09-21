//! Descriptor-bounded source roles for incoming relationship navigation.
//!
//! This memo contains descriptor identities only. Query evidence remains owned
//! by each answer, and callers retain the broad Incoming negative-search key.
use crate::KerMlQueries;
use agq_kernel::{ElementId, MetaclassId, PropertyId};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

type SourceRoles = BTreeMap<PropertyId, BTreeSet<MetaclassId>>;
pub(crate) type SourceRoleCache = BTreeMap<(MetaclassId, PropertyId), Arc<SourceRoles>>;

impl KerMlQueries<'_> {
    fn source_roles(&self, base: MetaclassId, source: PropertyId) -> Arc<SourceRoles> {
        let key = (base, source);
        if let Some(roles) = self
            .source_role_cache
            .lock()
            .expect("source role cache")
            .get(&key)
        {
            return roles.clone();
        }
        let registry = self.model().registry();
        let mut roles = SourceRoles::new();
        for class in registry.classes() {
            if registry.is_subtype(class.id, base).unwrap_or(false)
                && let Ok(Some(property)) = registry.resolve_property(class.id, source)
            {
                roles.entry(property.id).or_default().insert(class.id);
            }
        }
        let roles = Arc::new(roles);
        self.source_role_cache
            .lock()
            .expect("source role cache")
            .insert(key, roles.clone());
        roles
    }

    /// Enumerate only actual source endpoint buckets. For example,
    /// FeatureTyping::typedFeature replaces Specialization::specific; its
    /// general Type bucket is never examined by this outgoing query.
    pub(crate) fn incoming_source_relationships(
        &self,
        target: ElementId,
        base: MetaclassId,
        source_property: PropertyId,
    ) -> BTreeSet<ElementId> {
        let roles = self.source_roles(base, source_property);
        let mut relationships = BTreeSet::new();
        for (&property, classes) in roles.iter() {
            for reference in self.model().incoming_for_property(target, property) {
                // The bucket's descriptor identity alone does not prove the
                // relationship class or its effective redefinition context.
                if self
                    .model()
                    .element(reference.source)
                    .is_some_and(|record| classes.contains(&record.metaclass()))
                {
                    relationships.insert(reference.source);
                }
            }
        }
        relationships
    }
}

#[cfg(test)]
#[path = "../tests/unit/relationship_sources.rs"]
mod tests;
