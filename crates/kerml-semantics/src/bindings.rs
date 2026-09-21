//! Validated semantic anchors into canonical KerML library declarations.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{
    ElementId, LibraryId, MetaclassId,
    provenance::{DeclaredOrigin, Origin},
};
use std::collections::BTreeMap;

pub const BINDING_VERSION: &str = "agq-kerml-bindings/2";

/// KerML 1.0 semantic roles, not substitute declarations or OMG-defined IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardRole {
    Anything,
    DataValue,
    Things,
    DataValues,
    Naturals,
    Occurrence,
    Occurrences,
    Object,
    Objects,
    Link,
    BinaryLink,
    Links,
    SelfLinks,
    Performance,
    Performances,
    Evaluation,
    Evaluations,
    BooleanEvaluation,
    BooleanEvaluations,
    TrueEvaluations,
    FalseEvaluations,
    Metaobject,
    Metaobjects,
}
impl StandardRole {
    /// Binding contract: full declared path and exact concrete metaclass.
    pub fn specification(self) -> (&'static [&'static str], MetaclassId) {
        use StandardRole::*;
        match self {
            Anything => (&["Base", "Anything"], c::CLASSIFIER),
            DataValue => (&["Base", "DataValue"], c::DATA_TYPE),
            Things => (&["Base", "things"], c::FEATURE),
            DataValues => (&["Base", "dataValues"], c::FEATURE),
            Naturals => (&["Base", "naturals"], c::FEATURE),
            Occurrence => (&["Occurrences", "Occurrence"], c::CLASS),
            Occurrences => (&["Occurrences", "occurrences"], c::FEATURE),
            Object => (&["Objects", "Object"], c::STRUCTURE),
            Objects => (&["Objects", "objects"], c::FEATURE),
            Link => (&["Links", "Link"], c::ASSOCIATION),
            BinaryLink => (&["Links", "BinaryLink"], c::ASSOCIATION),
            Links => (&["Links", "links"], c::FEATURE),
            SelfLinks => (&["Links", "selfLinks"], c::FEATURE),
            Performance => (&["Performances", "Performance"], c::BEHAVIOR),
            Performances => (&["Performances", "performances"], c::STEP),
            Evaluation => (&["Performances", "Evaluation"], c::FUNCTION),
            Evaluations => (&["Performances", "evaluations"], c::EXPRESSION),
            BooleanEvaluation => (&["Performances", "BooleanEvaluation"], c::PREDICATE),
            BooleanEvaluations => (&["Performances", "booleanEvaluations"], c::EXPRESSION),
            TrueEvaluations => (&["Performances", "trueEvaluations"], c::EXPRESSION),
            FalseEvaluations => (&["Performances", "falseEvaluations"], c::EXPRESSION),
            Metaobject => (&["Metaobjects", "Metaobject"], c::METACLASS),
            Metaobjects => (&["Metaobjects", "metaobjects"], c::FEATURE),
        }
    }
    pub const ALL: [Self; 23] = [
        Self::Anything,
        Self::DataValue,
        Self::Things,
        Self::DataValues,
        Self::Naturals,
        Self::Occurrence,
        Self::Occurrences,
        Self::Object,
        Self::Objects,
        Self::Link,
        Self::BinaryLink,
        Self::Links,
        Self::SelfLinks,
        Self::Performance,
        Self::Performances,
        Self::Evaluation,
        Self::Evaluations,
        Self::BooleanEvaluation,
        Self::BooleanEvaluations,
        Self::TrueEvaluations,
        Self::FalseEvaluations,
        Self::Metaobject,
        Self::Metaobjects,
    ];
}

/// Validates the anchor declarations only. Overall library publication and
/// language validation remain separate obligations, including in a construction view.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandardKermlBindings {
    pub(crate) targets: BTreeMap<StandardRole, ElementId>,
    pub(crate) library: LibraryId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingError {
    Missing(StandardRole),
    Ambiguous(StandardRole),
    Incomplete(StandardRole),
    WrongMetaclass(StandardRole, ElementId),
    WrongLibrary(StandardRole, ElementId),
    Inaccessible(StandardRole, ElementId),
}
impl StandardKermlBindings {
    /// Resolve exact public owned declarations in the supplied library roots.
    /// Full paths, exact metaclasses, uniqueness and provenance are all required.
    pub fn validate(
        queries: &KerMlQueries<'_>,
        roots: &[ElementId],
        library: LibraryId,
    ) -> Result<Self, BindingError> {
        let mut targets = BTreeMap::new();
        for role in StandardRole::ALL {
            let (path, expected) = role.specification();
            let mut scopes = roots.to_vec();
            let mut target = None;
            for (index, segment) in path.iter().enumerate() {
                let mut candidates = vec![];
                for &scope in &scopes {
                    let members = queries.memberships(scope);
                    if members.completeness != Completeness::Complete {
                        return Err(BindingError::Incomplete(role));
                    }
                    for membership in members.value {
                        if !queries.is(membership, c::OWNING_MEMBERSHIP) {
                            continue;
                        }
                        let member = queries.member(membership);
                        if member.completeness != Completeness::Complete {
                            return Err(BindingError::Incomplete(role));
                        }
                        let Some(element) = member.value else {
                            return Err(BindingError::Incomplete(role));
                        };
                        let mut evidence = queries.result(());
                        if !matches!(queries.read_value(&mut evidence, element, p::ELEMENT_DECLARED_NAME), Some(agq_kernel::value::Value::String(name)) if name == segment)
                        {
                            continue;
                        }
                        if !queries.visible(
                            &mut evidence,
                            membership,
                            p::MEMBERSHIP_VISIBILITY,
                            MemberAccess::Public,
                        ) {
                            return Err(BindingError::Inaccessible(role, element));
                        }
                        if queries
                            .model()
                            .element(element)
                            .expect("member endpoint")
                            .origin()
                            != &Origin::Declared(DeclaredOrigin::StandardLibrary { library })
                        {
                            return Err(BindingError::WrongLibrary(role, element));
                        }
                        if queries
                            .model()
                            .element(membership)
                            .expect("owned membership")
                            .origin()
                            != &Origin::Declared(DeclaredOrigin::StandardLibrary { library })
                            || evidence.fact_origins.values().any(|origin| {
                                origin.as_ref()
                                    != &Origin::Declared(DeclaredOrigin::StandardLibrary {
                                        library,
                                    })
                            })
                        {
                            return Err(BindingError::WrongLibrary(role, membership));
                        }
                        if evidence.completeness != Completeness::Complete {
                            return Err(BindingError::Incomplete(role));
                        }
                        candidates.push(element);
                    }
                }
                let element = match candidates.as_slice() {
                    [] => return Err(BindingError::Missing(role)),
                    [element] => *element,
                    _ => return Err(BindingError::Ambiguous(role)),
                };
                let class = queries
                    .model()
                    .element(element)
                    .expect("candidate")
                    .metaclass();
                if (index + 1 == path.len() && class != expected)
                    || (index + 1 < path.len() && class != c::LIBRARY_PACKAGE)
                {
                    return Err(BindingError::WrongMetaclass(role, element));
                }
                scopes = vec![element];
                target = Some(element);
            }
            targets.insert(role, target.expect("nonempty role path"));
        }
        Ok(Self { targets, library })
    }
    pub fn get(&self, role: StandardRole) -> ElementId {
        self.targets[&role]
    }
    pub fn library(&self) -> LibraryId {
        self.library
    }
    pub fn iter(&self) -> impl Iterator<Item = (StandardRole, ElementId)> + '_ {
        self.targets.iter().map(|(r, id)| (*r, *id))
    }
}
