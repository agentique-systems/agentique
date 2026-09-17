//! Compare the published JSON serialization schema with XMI facts it exposes.
//! The schema is not MOF and is never used to fill in missing metamodel semantics.
use crate::{Result, ir::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossCheck {
    pub source: Source,
    pub classes: usize,
    pub enumerations: usize,
    pub owned_properties: usize,
    pub direct_generalizations: usize,
    pub nullable_data_scalars_with_positive_xmi_lower: Vec<String>,
    pub compared: Vec<String>,
    pub not_compared: Vec<String>,
}

pub fn check(model: &Metamodel, schema: &Value, source: Source) -> Result<CrossCheck> {
    let definitions = schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or("JSON cross-check: missing $defs")?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("JSON cross-check: unsupported schema dialect".into());
    }
    let mut names = BTreeMap::new();
    for classifier in model
        .classifiers
        .values()
        .filter(|c| matches!(c.entity.key.kind, Kind::Class | Kind::Enumeration))
    {
        if names
            .insert(classifier.entity.name.as_str(), classifier)
            .is_some()
        {
            return Err(
                "JSON cross-check: ambiguous simple-name projection; explicit mapping required"
                    .into(),
            );
        }
    }
    let mut expected_names: BTreeSet<_> = names.keys().copied().collect();
    expected_names.insert("Identified"); // JSON serialization helper, not a MOF class.
    let actual_names: BTreeSet<_> = definitions.keys().map(String::as_str).collect();
    if expected_names != actual_names {
        return Err("JSON cross-check mismatch: classifier definition set".into());
    }
    let mut report = CrossCheck {
        source, classes: 0, enumerations: 0, owned_properties: 0, direct_generalizations: 0,
        nullable_data_scalars_with_positive_xmi_lower: Vec::new(),
        compared: vec!["class/enum set and qualified schema URIs".into(), "enum literals".into(), "direct subtype alternatives".into(), "all directly owned class properties: presence, type/reference target, scalar/array shape, array bounds".into()],
        not_compared: vec!["abstractness; inherited/redefined property resolution".into(), "ordering; uniqueness; derivation; derived unions; subsetting; opposites; containment".into(), "scalar nullability as MOF lower bounds: JSON allows null for required scalar data values".into(), "operation/constraint semantics; defaults; isID and isReadOnly".into(), "association-owned ends (not serialized class properties)".into()],
    };
    for (name, classifier) in names {
        let definition = &definitions[name];
        let uri = format!("{}/{name}", model.source.metamodel_uri);
        if definition.get("$id") != Some(&json!(uri))
            || definition.get("title") != Some(&json!(name))
        {
            return Err(format!("JSON cross-check mismatch: identity {name}"));
        }
        if classifier.entity.key.kind == Kind::Enumeration {
            let mut expected = classifier
                .literals
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>();
            expected.sort();
            let mut actual: Vec<String> =
                serde_json::from_value(definition.get("enum").cloned().ok_or("JSON enum missing")?)
                    .map_err(|e| e.to_string())?;
            actual.sort();
            if actual != expected || definition.get("type") != Some(&json!("string")) {
                return Err(format!("JSON cross-check mismatch: enum {name}"));
            }
            report.enumerations += 1;
            continue;
        }
        let (object, subtype_refs) = if let Some(alternatives) = definition.get("anyOf") {
            let alternatives = alternatives.as_array().ok_or("invalid anyOf")?;
            let object = alternatives.first().ok_or("empty anyOf")?;
            let references = alternatives
                .iter()
                .skip(1)
                .map(|v| {
                    v.get("$ref")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .ok_or_else(|| "unsupported subtype alternative".into())
                })
                .collect::<Result<Vec<_>>>()?;
            (object, references)
        } else {
            (definition, Vec::new())
        };
        let mut expected_subtypes = model
            .classifiers
            .values()
            .filter(|c| {
                c.entity.key.kind == Kind::Class
                    && (c
                        .generalizations
                        .contains(&classifier.entity.key.external_id)
                        || c.generalizations.contains(&format!(
                            "{}#{}",
                            classifier.entity.key.source.artifact_uri,
                            classifier.entity.key.external_id
                        )))
            })
            .map(|c| format!("{}/{}", model.source.metamodel_uri, c.entity.name))
            .collect::<Vec<_>>();
        expected_subtypes.sort();
        let mut actual_subtypes = subtype_refs;
        actual_subtypes.sort();
        if actual_subtypes != expected_subtypes {
            return Err(format!(
                "JSON cross-check mismatch: direct subtypes of {name}"
            ));
        }
        report.direct_generalizations += expected_subtypes.len();
        let properties = object
            .get("properties")
            .and_then(Value::as_object)
            .ok_or("JSON class properties missing")?;
        if object.get("type") != Some(&json!("object"))
            || properties.get("@type").and_then(|p| p.get("const")) != Some(&json!(name))
        {
            return Err(format!("JSON cross-check mismatch: object kind {name}"));
        }
        for id in &classifier.properties {
            let property = &model.properties[id];
            let actual = properties.get(&property.entity.name).ok_or_else(|| {
                format!(
                    "JSON cross-check mismatch: missing {name}.{}",
                    property.entity.name
                )
            })?;
            let expected = property_schema(property, model)?;
            let mut compared = actual;
            if is_data(property, model)
                && matches!(property.upper, Upper::Finite(0 | 1))
                && let Some(variants) = actual.get("oneOf").and_then(Value::as_array)
            {
                if variants.len() != 2 || variants[1] != json!({"type": "null"}) {
                    return Err(format!(
                        "JSON cross-check: unsupported nullable data shape {name}.{}",
                        property.entity.name
                    ));
                }
                compared = &variants[0];
                if property.lower > 0 {
                    report
                        .nullable_data_scalars_with_positive_xmi_lower
                        .push(id.clone());
                }
            }
            if compared != &expected {
                return Err(format!(
                    "JSON cross-check mismatch: {name}.{}: expected {expected}, found {actual}",
                    property.entity.name
                ));
            }
            report.owned_properties += 1;
        }
        report.classes += 1;
    }
    Ok(report)
}

fn is_data(property: &Property, model: &Metamodel) -> bool {
    match &property.type_ref {
        TypeRef::External(_) => true,
        TypeRef::Local(id) => model.classifiers[id].entity.key.kind == Kind::Enumeration,
    }
}

fn property_schema(property: &Property, model: &Metamodel) -> Result<Value> {
    let (base, is_data) = match &property.type_ref {
        TypeRef::Local(id) => {
            let target = &model.classifiers[id];
            let uri = format!("{}/{}", model.source.metamodel_uri, target.entity.name);
            if target.entity.key.kind == Kind::Enumeration {
                (json!({"$ref": uri}), true)
            } else {
                (
                    json!({"$ref": format!("{}/Identified", model.source.metamodel_uri), "$comment": uri}),
                    false,
                )
            }
        }
        TypeRef::External(uri) => {
            let value_type = match uri.as_str() {
                "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#Boolean" => "boolean",
                "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#String" => "string",
                "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#Integer" => "integer",
                "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#Real" => "number",
                _ => {
                    return Err(format!(
                        "JSON cross-check: unsupported primitive projection {uri}"
                    ));
                }
            };
            (json!({"type": value_type}), true)
        }
    };
    if matches!(property.upper, Upper::Finite(0 | 1)) {
        Ok(if !is_data && property.lower == 0 {
            json!({"oneOf": [base, {"type": "null"}]})
        } else {
            base
        })
    } else {
        let mut result = json!({"type": "array", "items": base});
        if property.lower > 0 {
            result["minItems"] = json!(property.lower);
        }
        if let Upper::Finite(upper) = property.upper {
            result["maxItems"] = json!(upper);
        }
        Ok(result)
    }
}
