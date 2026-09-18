//! Authoring diagnostics do not decide whether a descriptor graph can be retained.
use super::*;

/// Typed descriptor identity, including independent identity namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DescriptorId {
    Metamodel(MetamodelId),
    Class(MetaclassId),
    Property(PropertyId),
    Association(AssociationId),
    Enumeration(EnumerationId),
    Literal(EnumerationLiteralId),
    Primitive(PrimitiveDomainId),
}

/// Exact, immutable source identity. A display name is never a review key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DescriptorSource {
    pub specification: String,
    pub version: String,
    pub artifact_uri: String,
    pub sha256: String,
    pub external_id: String,
    pub byte_range: [usize; 2],
}

/// Rules actually evaluated here; this is not a complete UML/CMOF validator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetamodelRule {
    ClassName,
    EnumerationLiteralName,
    EffectivePropertyConflict,
    SubsettedPropertyNames,
    SubsettingContext,
    SubsettingContract,
    RedefinitionContext,
    RedefinitionContract,
    RedefinitionCycle,
    DerivedUnion,
    OppositeType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCategory {
    Conformance,
    UnsupportedRuntimeSemantics,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticDisposition {
    Unreviewed,
    ReviewedBaselineAnomaly { evidence: String },
}

/// A review applies only to this exact rule, descriptor pair and two sources.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticReview {
    pub rule: MetamodelRule,
    pub subject: DescriptorId,
    pub related: DescriptorId,
    pub source: DescriptorSource,
    pub related_source: DescriptorSource,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetamodelDiagnostic {
    pub rule: MetamodelRule,
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub subject: DescriptorId,
    pub related_descriptors: BTreeSet<DescriptorId>,
    pub source: Option<DescriptorSource>,
    pub disposition: DiagnosticDisposition,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConformanceReport {
    pub diagnostics: BTreeSet<MetamodelDiagnostic>,
}
impl ConformanceReport {
    /// Reviewed errors still fail strict certification.
    pub fn require_conformance(&self) -> Result<(), ConformanceFailure> {
        let errors = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count();
        if errors == 0 {
            Ok(())
        } else {
            Err(ConformanceFailure { errors })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("metamodel conformance failed with {errors} errors (reviewed anomalies included)")]
pub struct ConformanceFailure {
    pub errors: usize,
}

/// Optional metamodel-authoring audit over a structurally registered graph.
pub struct MetamodelValidator;
impl MetamodelValidator {
    pub fn validate(registry: &MetamodelRegistry) -> ConformanceReport {
        let mut report = ConformanceReport::default();
        let mut add = |rule, subject, related: Option<DescriptorId>, category| {
            let source = registry.sources.get(&subject).cloned();
            let review = registry.reviews.iter().find(|r| {
                r.rule == rule
                    && r.subject == subject
                    && related == Some(r.related)
                    && source.as_ref() == Some(&r.source)
                    && registry.sources.get(&r.related) == Some(&r.related_source)
            });
            report.diagnostics.insert(MetamodelDiagnostic {
                rule,
                severity: DiagnosticSeverity::Error,
                category,
                subject,
                related_descriptors: related.into_iter().collect(),
                source,
                disposition: review.map_or(DiagnosticDisposition::Unreviewed, |r| {
                    DiagnosticDisposition::ReviewedBaselineAnomaly {
                        evidence: r.evidence.clone(),
                    }
                }),
            });
        };
        let mut names = BTreeMap::new();
        for c in registry.classes.values() {
            if let Some(first) = names.insert((c.metamodel, &c.package, &c.name), c.id) {
                add(
                    MetamodelRule::ClassName,
                    DescriptorId::Class(c.id),
                    Some(DescriptorId::Class(first)),
                    DiagnosticCategory::Conformance,
                );
            }
        }
        for enumeration in registry.enumerations.values() {
            let mut names = BTreeMap::new();
            for (&literal, name) in &enumeration.literals {
                if let Some(first) = names.insert(name, literal) {
                    add(
                        MetamodelRule::EnumerationLiteralName,
                        DescriptorId::Literal(literal),
                        Some(DescriptorId::Literal(first)),
                        DiagnosticCategory::Conformance,
                    );
                }
            }
        }
        for (&class, effective) in &registry.effective {
            let mut names = BTreeMap::new();
            for &id in effective {
                if let Some(first) = names.insert(&registry.properties[&id].name, id) {
                    add(
                        MetamodelRule::EffectivePropertyConflict,
                        DescriptorId::Class(class),
                        Some(DescriptorId::Property(first)),
                        DiagnosticCategory::Conformance,
                    );
                }
            }
        }
        for p in registry.properties.values() {
            let subject = DescriptorId::Property(p.id);
            if p.derived_union && !p.derived {
                add(
                    MetamodelRule::DerivedUnion,
                    subject,
                    None,
                    DiagnosticCategory::UnsupportedRuntimeSemantics,
                );
            }
            if let PropertyOwner::Class(owner) = p.owner {
                for opposite in &p.opposite_ends {
                    if registry.properties[opposite].value_kind != ValueKind::Reference(owner) {
                        add(
                            MetamodelRule::OppositeType,
                            subject,
                            Some(DescriptorId::Property(*opposite)),
                            DiagnosticCategory::Conformance,
                        );
                    }
                }
            }
            for base in &p.subsets {
                let b = &registry.properties[base];
                let related = Some(DescriptorId::Property(*base));
                if p.name == b.name {
                    add(
                        MetamodelRule::SubsettedPropertyNames,
                        subject,
                        related,
                        DiagnosticCategory::Conformance,
                    );
                }
                if !registry
                    .is_subtype(
                        registry.property_context(p).expect("integrity"),
                        registry.property_context(b).expect("integrity"),
                    )
                    .expect("integrity")
                {
                    add(
                        MetamodelRule::SubsettingContext,
                        subject,
                        related,
                        DiagnosticCategory::Conformance,
                    );
                }
                if !registry
                    .compatible_value(p.value_kind, b.value_kind)
                    .expect("integrity")
                    || !upper_contained(p, b)
                {
                    add(
                        MetamodelRule::SubsettingContract,
                        subject,
                        related,
                        DiagnosticCategory::Conformance,
                    );
                }
            }
            for base in &p.redefines {
                let b = &registry.properties[base];
                let related = Some(DescriptorId::Property(*base));
                let pc = registry.property_context(p).expect("integrity");
                let bc = registry.property_context(b).expect("integrity");
                if pc == bc || !registry.is_subtype(pc, bc).expect("integrity") {
                    add(
                        MetamodelRule::RedefinitionContext,
                        subject,
                        related,
                        DiagnosticCategory::Conformance,
                    );
                }
                if !registry.redefinition_contract_valid(p, b) {
                    add(
                        MetamodelRule::RedefinitionContract,
                        subject,
                        related,
                        DiagnosticCategory::Conformance,
                    );
                }
            }
        }
        let edges = registry
            .properties
            .iter()
            .map(|(&id, p)| (id, p.redefines.clone()))
            .collect();
        for id in crate::model::cyclic_nodes(&edges) {
            add(
                MetamodelRule::RedefinitionCycle,
                DescriptorId::Property(id),
                None,
                DiagnosticCategory::Conformance,
            );
        }
        for (&class, error) in &registry.effective_errors {
            let related = match error {
                MetamodelError::InvalidRedefinition { property, .. } => {
                    Some(DescriptorId::Property(*property))
                }
                _ => None,
            };
            add(
                MetamodelRule::EffectivePropertyConflict,
                DescriptorId::Class(class),
                related,
                DiagnosticCategory::UnsupportedRuntimeSemantics,
            );
        }
        report
    }
}

pub(super) fn upper_contained(p: &PropertyDescriptor, b: &PropertyDescriptor) -> bool {
    b.multiplicity
        .upper
        .is_none_or(|upper| p.multiplicity.upper.is_some_and(|n| n <= upper))
}
