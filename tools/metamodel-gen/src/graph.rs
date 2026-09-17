//! Temporary tooling projection spanning imports. Source-qualified map keys
//! preserve owning metamodel identities; descriptor keys are never rewritten.
use crate::{Result, ir::*};
use std::collections::BTreeMap;
pub fn qualified(model: &Metamodel, id: &str) -> String {
    if id.contains('#') {
        id.to_owned()
    } else {
        format!("{}#{id}", model.source.artifact_uri)
    }
}
pub fn combine(
    primary: &Metamodel,
    dependencies: &BTreeMap<String, Metamodel>,
) -> Result<Metamodel> {
    let mut result = Metamodel {
        format: "agentique-metamodel-graph/1".into(),
        source: primary.source.clone(),
        packages: BTreeMap::new(),
        classifiers: BTreeMap::new(),
        properties: BTreeMap::new(),
        retained: vec![],
    };
    for model in std::iter::once(primary).chain(dependencies.values()) {
        let q = |id: &str| qualified(model, id);
        let qs = |ids: &[String]| ids.iter().map(|id| q(id)).collect();
        for (id, package) in &model.packages {
            let mut package = package.clone();
            package.parent = package.parent.as_deref().map(q);
            package.imports = qs(&package.imports);
            if result.packages.insert(q(id), package).is_some() {
                return Err(format!("duplicate source-qualified package {}", q(id)));
            }
        }
        for (id, class) in &model.classifiers {
            let mut class = class.clone();
            class.package = q(&class.package);
            class.generalizations = qs(&class.generalizations);
            class.properties = qs(&class.properties);
            class.member_ends = qs(&class.member_ends);
            class.navigable_owned_ends = qs(&class.navigable_owned_ends);
            if result.classifiers.insert(q(id), class).is_some() {
                return Err(format!("duplicate source-qualified classifier {}", q(id)));
            }
        }
        for (id, property) in &model.properties {
            let mut property = property.clone();
            property.owner = q(&property.owner);
            property.redefines = qs(&property.redefines);
            property.subsets = qs(&property.subsets);
            property.opposite_ends = qs(&property.opposite_ends);
            property.association = property.association.as_deref().map(q);
            if let TypeRef::Local(id) = &property.type_ref {
                property.type_ref = TypeRef::Local(q(id));
            }
            if result.properties.insert(q(id), property).is_some() {
                return Err(format!("duplicate source-qualified property {}", q(id)));
            }
        }
    }
    for property in result.properties.values_mut() {
        if let TypeRef::External(uri) = &property.type_ref
            && result
                .classifiers
                .get(uri)
                .is_some_and(|c| c.entity.key.kind != Kind::PrimitiveType)
        {
            property.type_ref = TypeRef::Local(uri.clone());
        }
    }
    Ok(result)
}
