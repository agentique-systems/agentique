//! Concrete grammar to normative abstract-syntax classes, KerML 1.0 8.2.
use agq_kerml::classes as c;
use agq_kerml_syntax::production::{Node, Production as P};
use agq_kernel::MetaclassId;

pub(super) fn class(node: Node<'_>) -> Option<MetaclassId> {
    Some(match node.kind() {
        P::RootNamespace | P::Namespace => c::NAMESPACE,
        P::Package => c::PACKAGE,
        P::LibraryPackage => c::LIBRARY_PACKAGE,
        P::Type => c::TYPE,
        P::Classifier => c::CLASSIFIER,
        P::Class => c::CLASS,
        P::DataType => c::DATA_TYPE,
        P::Structure => c::STRUCTURE,
        P::Association => c::ASSOCIATION,
        P::AssociationStructure => c::ASSOCIATION_STRUCTURE,
        P::Behavior => c::BEHAVIOR,
        P::Function => c::FUNCTION,
        P::Predicate => c::PREDICATE,
        P::Metaclass => c::METACLASS,
        P::Interaction => c::INTERACTION,
        P::Feature
        | P::OwnedCrossFeature
        | P::ConnectorEnd
        | P::OwnedCrossMultiplicity
        | P::Argument
        | P::ArgumentExpression
        | P::MetadataArgument
        | P::TypeReference
        | P::EmptyFeature
        | P::PrimaryArgument
        | P::NonFeatureChainPrimaryArgument
        | P::BodyArgument
        | P::FunctionReferenceArgument
        | P::ConstructorResult
        | P::NamedArgument
        | P::FlowEnd
        | P::FlowFeature
        | P::PayloadFeature
        | P::FeatureChain => c::FEATURE,
        P::Step => c::STEP,
        P::Expression | P::ExpressionBody | P::FunctionReference => c::EXPRESSION,
        P::BooleanExpression => c::BOOLEAN_EXPRESSION,
        P::Invariant => c::INVARIANT,
        P::Connector => c::CONNECTOR,
        P::BindingConnector => c::BINDING_CONNECTOR,
        P::Succession => c::SUCCESSION,
        P::Flow => c::FLOW,
        P::SuccessionFlow => c::SUCCESSION_FLOW,
        P::Comment => c::COMMENT,
        P::Documentation => c::DOCUMENTATION,
        P::TextualRepresentation => c::TEXTUAL_REPRESENTATION,
        P::MultiplicityRange | P::OwnedMultiplicityRange => c::MULTIPLICITY_RANGE,
        P::MultiplicitySubset => c::MULTIPLICITY,
        P::NonFeatureMember
        | P::NamespaceFeatureMember
        | P::TypeFeatureMember
        | P::OwnedCrossFeatureMember
        | P::OwnedCrossMultiplicityMember
        | P::OwnedMultiplicity
        | P::MultiplicityExpressionMember
        | P::OwnedFeatureChainMember => c::OWNING_MEMBERSHIP,
        P::OwnedFeatureMember
        | P::OwnedExpressionReferenceMember
        | P::OwnedExpressionMember
        | P::ArgumentExpressionMember
        | P::FunctionReferenceMember
        | P::ExpressionBodyMember
        | P::SequenceExpressionListMember
        | P::PayloadFeatureMember
        | P::FlowFeatureMember
        | P::NamedArgumentMember => c::FEATURE_MEMBERSHIP,
        P::ConnectorEndMember | P::FlowEndMember => c::END_FEATURE_MEMBERSHIP,
        P::ReturnFeatureMember | P::EmptyResultMember | P::ConstructorResultMember => {
            c::RETURN_PARAMETER_MEMBERSHIP
        }
        P::ResultExpressionMember => c::RESULT_EXPRESSION_MEMBERSHIP,
        P::ArgumentMember
        | P::MetadataArgumentMember
        | P::TypeReferenceMember
        | P::PrimaryArgumentMember
        | P::NonFeatureChainPrimaryArgumentMember
        | P::BodyArgumentMember
        | P::FunctionReferenceArgumentMember => c::PARAMETER_MEMBERSHIP,
        P::TypeResultMember => c::RETURN_PARAMETER_MEMBERSHIP,
        P::AliasMember
        | P::FeatureReferenceMember
        | P::ElementReferenceMember
        | P::InvocationTypeMember => c::MEMBERSHIP,
        P::InstantiatedTypeMember if node.child(P::OwnedFeatureChainMember).is_none() => {
            c::MEMBERSHIP
        }
        P::MembershipImport => c::MEMBERSHIP_IMPORT,
        P::NamespaceImport => c::NAMESPACE_IMPORT,
        P::Specialization | P::OwnedSpecialization => c::SPECIALIZATION,
        P::Subclassification | P::OwnedSubclassification => c::SUBCLASSIFICATION,
        P::FeatureTyping | P::OwnedFeatureTyping | P::ReferenceTyping => c::FEATURE_TYPING,
        P::Subsetting | P::OwnedSubsetting => c::SUBSETTING,
        P::Redefinition
        | P::OwnedRedefinition
        | P::ParameterRedefinition
        | P::FlowFeatureRedefinition => c::REDEFINITION,
        P::OwnedReferenceSubsetting => c::REFERENCE_SUBSETTING,
        P::OwnedCrossSubsetting => c::CROSS_SUBSETTING,
        P::Conjugation | P::OwnedConjugation => c::CONJUGATION,
        P::Disjoining | P::OwnedDisjoining => c::DISJOINING,
        P::Unioning => c::UNIONING,
        P::Intersecting => c::INTERSECTING,
        P::Differencing => c::DIFFERENCING,
        P::OwnedFeatureChaining => c::FEATURE_CHAINING,
        P::FeatureInverting | P::OwnedFeatureInverting => c::FEATURE_INVERTING,
        P::TypeFeaturing | P::OwnedTypeFeaturing => c::TYPE_FEATURING,
        P::FeatureValue
        | P::ArgumentValue
        | P::ArgumentExpressionValue
        | P::MetadataValue
        | P::PrimaryArgumentValue
        | P::NonFeatureChainPrimaryArgumentValue
        | P::BodyArgumentValue
        | P::FunctionReferenceArgumentValue => c::FEATURE_VALUE,
        P::ConditionalExpression
        | P::ConditionalBinaryOperatorExpression
        | P::BinaryOperatorExpression
        | P::UnaryOperatorExpression
        | P::ClassificationExpression
        | P::MetaclassificationExpression
        | P::ExtentExpression
        | P::BracketExpression
        | P::SequenceOperatorExpression => c::OPERATOR_EXPRESSION,
        P::FeatureChainExpression => c::FEATURE_CHAIN_EXPRESSION,
        P::SelectExpression => c::SELECT_EXPRESSION,
        P::CollectExpression => c::COLLECT_EXPRESSION,
        P::IndexExpression => c::INDEX_EXPRESSION,
        P::OwnedExpressionReference
        | P::FeatureReferenceExpression
        | P::FunctionReferenceExpression
        | P::BodyExpression => c::FEATURE_REFERENCE_EXPRESSION,
        P::InvocationExpression | P::FunctionOperationExpression => c::INVOCATION_EXPRESSION,
        P::ConstructorExpression => c::CONSTRUCTOR_EXPRESSION,
        P::MetadataAccessExpression | P::MetadataReference => c::METADATA_ACCESS_EXPRESSION,
        P::NullExpression => c::NULL_EXPRESSION,
        P::LiteralBoolean => c::LITERAL_BOOLEAN,
        P::LiteralInteger => c::LITERAL_INTEGER,
        P::LiteralReal => c::LITERAL_RATIONAL,
        P::LiteralInfinity => c::LITERAL_INFINITY,
        P::LiteralString => c::LITERAL_STRING,
        _ => return None,
    })
}
