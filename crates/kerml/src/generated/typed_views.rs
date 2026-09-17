// GENERATED FILE. DO NOT EDIT.
// Generator: agq-kerml-typed-views/1
// Reproduce: cargo run --locked --offline -p agq-metamodel-gen
// KerML.xmi SHA-256: 45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466
// PrimitiveTypes.xmi SHA-256: 62d12217fcd26037fc917709e2a896600af574efd6412c110e2a711395a69849
// KerML.json SHA-256: e454fe4b7c04f3d95874b6c1a4e6ef056ea5874c71fd3d17e3180319c2f58ab2
// Golden SHA-256: ca76228dfbf88d66f028a2e504b39025dd0533b1924ed90956f46415b69c81c8

/// Identity of the pinned KerML 1.0 metamodel.
#[rustfmt::skip]
pub mod metamodel {
    pub const KERML: agq_kernel::MetamodelId = agq_kernel::MetamodelId::from_u128(0x5714ddced06b5e85a2fd3079c545d33d);
}
/// Class IDs from source-qualified normative keys; Rust names are conveniences.
#[rustfmt::skip]
pub mod classes {
    /// XMI identity: `Core-Features-CrossSubsetting`.
    pub const CROSS_SUBSETTING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x25ad6dbd70505f1593ab9897bb69155c);
    /// XMI identity: `Core-Features-Feature`.
    pub const FEATURE: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xfc178fd99abc598fa3bbfcefa8f9a0ed);
    /// XMI identity: `Core-Features-FeatureChaining`.
    pub const FEATURE_CHAINING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x3105e9a1c92a5921874512b7c3bcd177);
    /// XMI identity: `Core-Features-FeatureInverting`.
    pub const FEATURE_INVERTING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x828ba97e2c305e94a33014b83529b635);
    /// XMI identity: `Core-Features-FeatureTyping`.
    pub const FEATURE_TYPING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x62dc33ff9e92557eb7a9562372f000f3);
    /// XMI identity: `Core-Features-Redefinition`.
    pub const REDEFINITION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x07e37588467a5a04aa3d8146336c5c52);
    /// XMI identity: `Core-Features-ReferenceSubsetting`.
    pub const REFERENCE_SUBSETTING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x3a92734cbf785e14a1111296c7098905);
    /// XMI identity: `Core-Features-Subsetting`.
    pub const SUBSETTING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x149853436f6c57f7899389b86f86101c);
    /// XMI identity: `Core-Features-TypeFeaturing`.
    pub const TYPE_FEATURING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x45a49e2f36dc5da2b77f617eb53e0e8c);
    /// XMI identity: `Core-Types-Conjugation`.
    pub const CONJUGATION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x7f1b2266df8a5bb2bffae14545bcf943);
    /// XMI identity: `Core-Types-Differencing`.
    pub const DIFFERENCING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xb1998683180c5e6c97b22b0762d3453b);
    /// XMI identity: `Core-Types-Disjoining`.
    pub const DISJOINING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x40e1cb5557e75104a710659b00bbd80f);
    /// XMI identity: `Core-Types-FeatureMembership`.
    pub const FEATURE_MEMBERSHIP: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xba6b135b94d45ce29c91f19f8a82e270);
    /// XMI identity: `Core-Types-Intersecting`.
    pub const INTERSECTING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xba8c36af85c95dc188c1b1ed2cb51438);
    /// XMI identity: `Core-Types-Multiplicity`.
    pub const MULTIPLICITY: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x967bd55ad5a05d999c5e2caf2ace1536);
    /// XMI identity: `Core-Types-Specialization`.
    pub const SPECIALIZATION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x09c22c5242065efa94a6733e3ad7936d);
    /// XMI identity: `Core-Types-Type`.
    pub const TYPE: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x15066a0c72215fd2a941648d0876392f);
    /// XMI identity: `Core-Types-Unioning`.
    pub const UNIONING: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x93e19bf0a21159c4a35a79e45f449857);
    /// XMI identity: `Root-Annotations-AnnotatingElement`.
    pub const ANNOTATING_ELEMENT: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xc63d2a0b700556159488349c203357a6);
    /// XMI identity: `Root-Annotations-Annotation`.
    pub const ANNOTATION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x0de89ff520e95534b45d771d6a3d19d9);
    /// XMI identity: `Root-Annotations-Comment`.
    pub const COMMENT: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xccd9179b74075ec090a571f97c2ba994);
    /// XMI identity: `Root-Annotations-Documentation`.
    pub const DOCUMENTATION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x9a61e02e56ab509589051158739d3dff);
    /// XMI identity: `Root-Annotations-TextualRepresentation`.
    pub const TEXTUAL_REPRESENTATION: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x33bfc0cdd5b95121b0f46b1400b6c232);
    /// XMI identity: `Root-Elements-Element`.
    pub const ELEMENT: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x7e47b9cf8e2d5a5fa2116752552a45e7);
    /// XMI identity: `Root-Elements-Relationship`.
    pub const RELATIONSHIP: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x7344b37d692d530390fd4e809e7aba82);
    /// XMI identity: `Root-Namespaces-Import`.
    pub const IMPORT: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xd18f583b6c8252d694bb36fc571c898f);
    /// XMI identity: `Root-Namespaces-Membership`.
    pub const MEMBERSHIP: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xf1d3ce0b983455269185b2abc548b909);
    /// XMI identity: `Root-Namespaces-Namespace`.
    pub const NAMESPACE: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x67f18d51d69256c88de27494d67e8b81);
    /// XMI identity: `Root-Namespaces-OwningMembership`.
    pub const OWNING_MEMBERSHIP: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0xb975bd0e083a573cbd4ce779ca389eae);
}

/// Property IDs, including association-owned metadata ends (not class slots).
#[rustfmt::skip]
pub mod properties {
    /// XMI identity: `Core-Features-A_chainingFeature_chainedFeature-chainedFeature`.
    pub const A_CHAINING_FEATURE_CHAINED_FEATURE_CHAINED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xe19040ac55ba550289c09463793afac2);
    /// XMI identity: `Core-Features-A_chainingFeature_chainedFeatureChaining-chainedFeatureChaining`.
    pub const A_CHAINING_FEATURE_CHAINED_FEATURE_CHAINING_CHAINED_FEATURE_CHAINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x87bda3b24d1e5fb8b157c5948275c06a);
    /// XMI identity: `Core-Features-A_crossFeature_featureCrossing-featureCrossing`.
    pub const A_CROSS_FEATURE_FEATURE_CROSSING_FEATURE_CROSSING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x475a5d14ca06516c959de0b60c6a55a7);
    /// XMI identity: `Core-Features-A_crossedFeature_crossSupersetting-crossSupersetting`.
    pub const A_CROSSED_FEATURE_CROSS_SUPERSETTING_CROSS_SUPERSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0f2d00ba37005c8f9fa5250caa96be9d);
    /// XMI identity: `Core-Features-A_featureOfType_typeFeaturing-typeFeaturing`.
    pub const A_FEATURE_OF_TYPE_TYPE_FEATURING_TYPE_FEATURING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa606b07049745ab9b434bbc1aababaca);
    /// XMI identity: `Core-Features-A_featureTarget_baseFeature-baseFeature`.
    pub const A_FEATURE_TARGET_BASE_FEATURE_BASE_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0fe62f0df93c55feb250d49f9e0d6a10);
    /// XMI identity: `Core-Features-A_featuringType_featureOfType-featureOfType`.
    pub const A_FEATURING_TYPE_FEATURE_OF_TYPE_FEATURE_OF_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4acb4540abac5fd884a2628ae18f8e5d);
    /// XMI identity: `Core-Features-A_featuringType_typeFeaturingOfType-typeFeaturingOfType`.
    pub const A_FEATURING_TYPE_TYPE_FEATURING_OF_TYPE_TYPE_FEATURING_OF_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa0503278d036505b988482ae84f2fc2e);
    /// XMI identity: `Core-Features-A_invertingFeatureInverting_featureInverted-invertingFeatureInverting`.
    pub const A_INVERTING_FEATURE_INVERTING_FEATURE_INVERTED_INVERTING_FEATURE_INVERTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xbed4371a58f859e3b1108cefd49f6d38);
    /// XMI identity: `Core-Features-A_invertingFeature_invertedFeatureInverting-invertedFeatureInverting`.
    pub const A_INVERTING_FEATURE_INVERTED_FEATURE_INVERTING_INVERTED_FEATURE_INVERTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x12b0c6f8db5254cdb2ee5cb430629db8);
    /// XMI identity: `Core-Features-A_multiplicity_typeWithMultiplicity-typeWithMultiplicity`.
    pub const A_MULTIPLICITY_TYPE_WITH_MULTIPLICITY_TYPE_WITH_MULTIPLICITY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8463fb96b8a7502eac441549734c1f5d);
    /// XMI identity: `Core-Features-A_ownedRedefinition_owningFeature-owningFeature`.
    pub const A_OWNED_REDEFINITION_OWNING_FEATURE_OWNING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x232112d88b75513ea51801d9071d1bed);
    /// XMI identity: `Core-Features-A_redefinedFeature_redefining-redefining`.
    pub const A_REDEFINED_FEATURE_REDEFINING_REDEFINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x894224846bfd5e3785f8a686f1b34840);
    /// XMI identity: `Core-Features-A_redefiningFeature_redefinition-redefinition`.
    pub const A_REDEFINING_FEATURE_REDEFINITION_REDEFINITION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1e81b473e56c570b9c0b3d9c8bfad1a5);
    /// XMI identity: `Core-Features-A_referencedFeature_referencing-referencing`.
    pub const A_REFERENCED_FEATURE_REFERENCING_REFERENCING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x39160a8254df5bae9d015cf34daaaf5b);
    /// XMI identity: `Core-Features-A_subsettedFeature_supersetting-supersetting`.
    pub const A_SUBSETTED_FEATURE_SUPERSETTING_SUPERSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x3a4e7938003c58a9bf9377b58735c397);
    /// XMI identity: `Core-Features-A_subsettingFeature_subsetting-subsetting`.
    pub const A_SUBSETTING_FEATURE_SUBSETTING_SUBSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xbc06b9aa566252f3b8e9eca1741e9802);
    /// XMI identity: `Core-Features-A_type_typingByType-typingByType`.
    pub const A_TYPE_TYPING_BY_TYPE_TYPING_BY_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x9666e8a5dd9254b3921b738a2609ecb3);
    /// XMI identity: `Core-Features-A_typedFeature_type-typedFeature`.
    pub const A_TYPED_FEATURE_TYPE_TYPED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa5e5504d7a945d74ba9d0c5da4a75b9a);
    /// XMI identity: `Core-Features-A_typing_typedFeature-typing`.
    pub const A_TYPING_TYPED_FEATURE_TYPING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc7df9e52010f548ab7f174ff8e60b4ad);
    /// XMI identity: `Core-Features-CrossSubsetting-crossedFeature`.
    pub const CROSS_SUBSETTING_CROSSED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x26017bacc30251f9bcd361d010764f06);
    /// XMI identity: `Core-Features-CrossSubsetting-crossingFeature`.
    pub const CROSS_SUBSETTING_CROSSING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfa268923d00c51468e6d7841730ef9b8);
    /// XMI identity: `Core-Features-Feature-chainingFeature`.
    pub const FEATURE_CHAINING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc1c25cfee40454048c9d9442ef43f5e8);
    /// XMI identity: `Core-Features-Feature-crossFeature`.
    pub const FEATURE_CROSS_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf0a8136a84e95ae28fb78175237c6832);
    /// XMI identity: `Core-Features-Feature-direction`.
    pub const FEATURE_DIRECTION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x9cddc57a30c25564a63a8982115d1a56);
    /// XMI identity: `Core-Features-Feature-endOwningType`.
    pub const FEATURE_END_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd6bd6367ecbd54f59d492398fbe88481);
    /// XMI identity: `Core-Features-Feature-featureTarget`.
    pub const FEATURE_FEATURE_TARGET: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd64267fa635c5ad3aa43d00660db991a);
    /// XMI identity: `Core-Features-Feature-featuringType`.
    pub const FEATURE_FEATURING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x51d01fcf83125d059ba1983da155c286);
    /// XMI identity: `Core-Features-Feature-isComposite`.
    pub const FEATURE_IS_COMPOSITE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xe982a99589285bcda401b676b4e2228b);
    /// XMI identity: `Core-Features-Feature-isConstant`.
    pub const FEATURE_IS_CONSTANT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7949c65575d7559583258fd01bd475e1);
    /// XMI identity: `Core-Features-Feature-isDerived`.
    pub const FEATURE_IS_DERIVED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x474b15e67d995bb9a9389953d72ccb8a);
    /// XMI identity: `Core-Features-Feature-isEnd`.
    pub const FEATURE_IS_END: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x007092e95bfe5b8795b109d80e23148e);
    /// XMI identity: `Core-Features-Feature-isOrdered`.
    pub const FEATURE_IS_ORDERED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0147a6e7bbc75323ac9cdf87cba0fcd7);
    /// XMI identity: `Core-Features-Feature-isPortion`.
    pub const FEATURE_IS_PORTION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x5ea55dd85fd1574ab2863fdd5a863fe7);
    /// XMI identity: `Core-Features-Feature-isUnique`.
    pub const FEATURE_IS_UNIQUE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x3aef3903cb905765b4f94874cc06531e);
    /// XMI identity: `Core-Features-Feature-isVariable`.
    pub const FEATURE_IS_VARIABLE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1b641cd3a32a5c7099df1343eee052fb);
    /// XMI identity: `Core-Features-Feature-ownedCrossSubsetting`.
    pub const FEATURE_OWNED_CROSS_SUBSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x235787ba96075d3499cfb50b79a23dfc);
    /// XMI identity: `Core-Features-Feature-ownedFeatureChaining`.
    pub const FEATURE_OWNED_FEATURE_CHAINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x302445ade0035193b3a7a79cf9f5ee4b);
    /// XMI identity: `Core-Features-Feature-ownedFeatureInverting`.
    pub const FEATURE_OWNED_FEATURE_INVERTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4f216ed0a9dd5f408af9dc7643322201);
    /// XMI identity: `Core-Features-Feature-ownedRedefinition`.
    pub const FEATURE_OWNED_REDEFINITION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xcb566800577658ba9cd30d34d7ada0af);
    /// XMI identity: `Core-Features-Feature-ownedReferenceSubsetting`.
    pub const FEATURE_OWNED_REFERENCE_SUBSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd0fa8bb64d6951a99c9cd8c04a873c96);
    /// XMI identity: `Core-Features-Feature-ownedSubsetting`.
    pub const FEATURE_OWNED_SUBSETTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x39d72cd5d3fd5701b98846ab14c63ea3);
    /// XMI identity: `Core-Features-Feature-ownedTypeFeaturing`.
    pub const FEATURE_OWNED_TYPE_FEATURING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x98ecb052842b5a4fb775662f94b06f27);
    /// XMI identity: `Core-Features-Feature-ownedTyping`.
    pub const FEATURE_OWNED_TYPING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x34e3cfd35612541ab09207a450cb6c93);
    /// XMI identity: `Core-Features-Feature-owningFeatureMembership`.
    pub const FEATURE_OWNING_FEATURE_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd2331d8287f85d47ae399b88b9a43b55);
    /// XMI identity: `Core-Features-Feature-owningType`.
    pub const FEATURE_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4a25d6061121537eb8dc5e187368f19e);
    /// XMI identity: `Core-Features-Feature-type`.
    pub const FEATURE_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4907fddcc26b53508220a4b13b082f39);
    /// XMI identity: `Core-Features-FeatureChaining-chainingFeature`.
    pub const FEATURE_CHAINING_CHAINING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xdb4ec700c48153edb0b00e83e5fb9619);
    /// XMI identity: `Core-Features-FeatureChaining-featureChained`.
    pub const FEATURE_CHAINING_FEATURE_CHAINED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x2311d3149a3d5a9784182628cf25ef2e);
    /// XMI identity: `Core-Features-FeatureInverting-featureInverted`.
    pub const FEATURE_INVERTING_FEATURE_INVERTED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf9b2703ca23c5180a939fc2530bdceeb);
    /// XMI identity: `Core-Features-FeatureInverting-invertingFeature`.
    pub const FEATURE_INVERTING_INVERTING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfa70a2d47a805b7a9fb8f9fd2fa1bb9b);
    /// XMI identity: `Core-Features-FeatureInverting-owningFeature`.
    pub const FEATURE_INVERTING_OWNING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x619eb3e4ce9e56edad29192dbd68eb21);
    /// XMI identity: `Core-Features-FeatureTyping-owningFeature`.
    pub const FEATURE_TYPING_OWNING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1d9ba0729f4c59668b95636fd0715e06);
    /// XMI identity: `Core-Features-FeatureTyping-type`.
    pub const FEATURE_TYPING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0bdf3b5b03eb5d5d8012c145b713a986);
    /// XMI identity: `Core-Features-FeatureTyping-typedFeature`.
    pub const FEATURE_TYPING_TYPED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8677e0e537a95db8bf0e804cc15a45f6);
    /// XMI identity: `Core-Features-Redefinition-redefinedFeature`.
    pub const REDEFINITION_REDEFINED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc7c0ed4bfe49572e80d82537bc910c52);
    /// XMI identity: `Core-Features-Redefinition-redefiningFeature`.
    pub const REDEFINITION_REDEFINING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0e06593e069e5c27af0572fc27199090);
    /// XMI identity: `Core-Features-ReferenceSubsetting-referencedFeature`.
    pub const REFERENCE_SUBSETTING_REFERENCED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa50148e3e9b1583ebd5bd9d35f346af9);
    /// XMI identity: `Core-Features-ReferenceSubsetting-referencingFeature`.
    pub const REFERENCE_SUBSETTING_REFERENCING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x79d28cf2e3e65e898d2681462d39c1d6);
    /// XMI identity: `Core-Features-Subsetting-owningFeature`.
    pub const SUBSETTING_OWNING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x33c4ba658f3955a9ab56bb04ec8f7bf4);
    /// XMI identity: `Core-Features-Subsetting-subsettedFeature`.
    pub const SUBSETTING_SUBSETTED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xba5e3468f3f05830bb84f9efff91ed06);
    /// XMI identity: `Core-Features-Subsetting-subsettingFeature`.
    pub const SUBSETTING_SUBSETTING_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xab6ec0b052d2527fbde64ccdec5d61a4);
    /// XMI identity: `Core-Features-TypeFeaturing-featureOfType`.
    pub const TYPE_FEATURING_FEATURE_OF_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x5bb79164ce205e97a8a95a3516b17cb5);
    /// XMI identity: `Core-Features-TypeFeaturing-featuringType`.
    pub const TYPE_FEATURING_FEATURING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf2c1c05ba7ec5536905b7686c62613e8);
    /// XMI identity: `Core-Features-TypeFeaturing-owningFeatureOfType`.
    pub const TYPE_FEATURING_OWNING_FEATURE_OF_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x48e7c0e4ebdd560a9e60b295bf58fd5d);
    /// XMI identity: `Core-Types-A_conjugatedType_conjugator-conjugator`.
    pub const A_CONJUGATED_TYPE_CONJUGATOR_CONJUGATOR: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1c9348e9e4d552139b2b626a0845913b);
    /// XMI identity: `Core-Types-A_differencingType_differencedDifferencing-differencedDifferencing`.
    pub const A_DIFFERENCING_TYPE_DIFFERENCED_DIFFERENCING_DIFFERENCED_DIFFERENCING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x41b64277bc6756ffbc91bf1bd6e5b053);
    /// XMI identity: `Core-Types-A_differencingType_differencedType-differencedType`.
    pub const A_DIFFERENCING_TYPE_DIFFERENCED_TYPE_DIFFERENCED_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1403b8a6afee55debf792105af0517e9);
    /// XMI identity: `Core-Types-A_directedFeature_typeWithDirectedFeature-typeWithDirectedFeature`.
    pub const A_DIRECTED_FEATURE_TYPE_WITH_DIRECTED_FEATURE_TYPE_WITH_DIRECTED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x687a89b735ec5a239deb19c65edc6ece);
    /// XMI identity: `Core-Types-A_disjoiningTypeDisjoining_typeDisjoined-disjoiningTypeDisjoining`.
    pub const A_DISJOINING_TYPE_DISJOINING_TYPE_DISJOINED_DISJOINING_TYPE_DISJOINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x48e0fa19cebb585695c5577f20df8703);
    /// XMI identity: `Core-Types-A_disjoiningType_disjoinedTypeDisjoining-disjoinedTypeDisjoining`.
    pub const A_DISJOINING_TYPE_DISJOINED_TYPE_DISJOINING_DISJOINED_TYPE_DISJOINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd494a354135f5bd9aa6e303255ab3dd9);
    /// XMI identity: `Core-Types-A_endFeature_typeWithEndFeature-typeWithEndFeature`.
    pub const A_END_FEATURE_TYPE_WITH_END_FEATURE_TYPE_WITH_END_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc1278a5f6960550092eb46e4084de15e);
    /// XMI identity: `Core-Types-A_featureMembership_type-type`.
    pub const A_FEATURE_MEMBERSHIP_TYPE_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd0c836272e0e5217b6e71a06939be0bb);
    /// XMI identity: `Core-Types-A_general_generalization-generalization`.
    pub const A_GENERAL_GENERALIZATION_GENERALIZATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfebff9812dc758739482e0797ab0fc73);
    /// XMI identity: `Core-Types-A_inheritedFeature_inheritingType-inheritingType`.
    pub const A_INHERITED_FEATURE_INHERITING_TYPE_INHERITING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd3d526327c93569d91772d078b54bd22);
    /// XMI identity: `Core-Types-A_inheritedMembership_inheritingType-inheritingType`.
    pub const A_INHERITED_MEMBERSHIP_INHERITING_TYPE_INHERITING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc5a5e0fafd9f5c169c024f5252784114);
    /// XMI identity: `Core-Types-A_input_typeWithInput-typeWithInput`.
    pub const A_INPUT_TYPE_WITH_INPUT_TYPE_WITH_INPUT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x52f6a4ac9a4f5596934ff511228b37eb);
    /// XMI identity: `Core-Types-A_intersectingType_intersectedIntersecting-intersectedIntersecting`.
    pub const A_INTERSECTING_TYPE_INTERSECTED_INTERSECTING_INTERSECTED_INTERSECTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x90b73611e037563b916efb3ff8f7827c);
    /// XMI identity: `Core-Types-A_intersectingType_intersectedType-intersectedType`.
    pub const A_INTERSECTING_TYPE_INTERSECTED_TYPE_INTERSECTED_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x5d0c710e798a5f299f6b7403137ffd37);
    /// XMI identity: `Core-Types-A_originalType_conjugation-conjugation`.
    pub const A_ORIGINAL_TYPE_CONJUGATION_CONJUGATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x90d39c86beba57538d6a54fc5e6e321e);
    /// XMI identity: `Core-Types-A_output_typeWithOutput-typeWithOutput`.
    pub const A_OUTPUT_TYPE_WITH_OUTPUT_TYPE_WITH_OUTPUT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4fee70cf6b5954a087b8c15cede398e2);
    /// XMI identity: `Core-Types-A_specific_specialization-specialization`.
    pub const A_SPECIFIC_SPECIALIZATION_SPECIALIZATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x72755f3c2e0250b0b7decb388ec6ac15);
    /// XMI identity: `Core-Types-A_typeWithFeature_feature-typeWithFeature`.
    pub const A_TYPE_WITH_FEATURE_FEATURE_TYPE_WITH_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x507bef3062645fab8315d6daed8eef09);
    /// XMI identity: `Core-Types-A_unioningType_unionedType-unionedType`.
    pub const A_UNIONING_TYPE_UNIONED_TYPE_UNIONED_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8c57df4ce97755fe8a8c6221aeda7676);
    /// XMI identity: `Core-Types-A_unioningType_unionedUnioning-unionedUnioning`.
    pub const A_UNIONING_TYPE_UNIONED_UNIONING_UNIONED_UNIONING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7cfca72deb2e58168c22d71b05a800ca);
    /// XMI identity: `Core-Types-Conjugation-conjugatedType`.
    pub const CONJUGATION_CONJUGATED_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0909c810fc645f009c7e7b01a1cdf4da);
    /// XMI identity: `Core-Types-Conjugation-originalType`.
    pub const CONJUGATION_ORIGINAL_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x472cbc743d5e5744a4330f63391d9ce0);
    /// XMI identity: `Core-Types-Conjugation-owningType`.
    pub const CONJUGATION_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x24bd06389d2f54dab82ec4775fa588a9);
    /// XMI identity: `Core-Types-Differencing-differencingType`.
    pub const DIFFERENCING_DIFFERENCING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x892118fec15f5b9b907a19f7be6eba04);
    /// XMI identity: `Core-Types-Differencing-typeDifferenced`.
    pub const DIFFERENCING_TYPE_DIFFERENCED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x08a34c7b638d549cb5e59e17ab324677);
    /// XMI identity: `Core-Types-Disjoining-disjoiningType`.
    pub const DISJOINING_DISJOINING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8687a6450d9452ccbb116616a7946fe5);
    /// XMI identity: `Core-Types-Disjoining-owningType`.
    pub const DISJOINING_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x95fc6063e9185d43ba32b37da9f3632d);
    /// XMI identity: `Core-Types-Disjoining-typeDisjoined`.
    pub const DISJOINING_TYPE_DISJOINED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd79aaadb421d51f5800fbaf2ec7f0092);
    /// XMI identity: `Core-Types-FeatureMembership-ownedMemberFeature`.
    pub const FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x71e97e993dc551b38c79a42c3abff880);
    /// XMI identity: `Core-Types-FeatureMembership-owningType`.
    pub const FEATURE_MEMBERSHIP_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xb0e7b15db5ef5f6b9c821b97c69b1ff8);
    /// XMI identity: `Core-Types-Intersecting-intersectingType`.
    pub const INTERSECTING_INTERSECTING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7bcb567f9d4554deb02c7b3fe29bde86);
    /// XMI identity: `Core-Types-Intersecting-typeIntersected`.
    pub const INTERSECTING_TYPE_INTERSECTED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x2ac04b77b5bc539f82501a61e5e6aed1);
    /// XMI identity: `Core-Types-Specialization-general`.
    pub const SPECIALIZATION_GENERAL: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4fbf926bf90754c29fba9953bbe4e5af);
    /// XMI identity: `Core-Types-Specialization-owningType`.
    pub const SPECIALIZATION_OWNING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x14e158c3dea15afc97562fb850863a15);
    /// XMI identity: `Core-Types-Specialization-specific`.
    pub const SPECIALIZATION_SPECIFIC: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x95d8b51c954a56beb2d57bc24e9ec255);
    /// XMI identity: `Core-Types-Type-differencingType`.
    pub const TYPE_DIFFERENCING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x03dcdad0bb4a518ea45249fa1782b219);
    /// XMI identity: `Core-Types-Type-directedFeature`.
    pub const TYPE_DIRECTED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xb460bb14213e5c0088bc8a6e850bac14);
    /// XMI identity: `Core-Types-Type-endFeature`.
    pub const TYPE_END_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfb220076c84a5368867c307e316a5989);
    /// XMI identity: `Core-Types-Type-feature`.
    pub const TYPE_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6262d8a7407c5ddf8944041e3bd275e9);
    /// XMI identity: `Core-Types-Type-featureMembership`.
    pub const TYPE_FEATURE_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd1759ae856185a1782b01fabfe4a8d5e);
    /// XMI identity: `Core-Types-Type-inheritedFeature`.
    pub const TYPE_INHERITED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x00bba1d8caa553c6b636f8d38b4c1447);
    /// XMI identity: `Core-Types-Type-inheritedMembership`.
    pub const TYPE_INHERITED_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6a25832a17c65cdc99fe24345a507262);
    /// XMI identity: `Core-Types-Type-input`.
    pub const TYPE_INPUT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x80f58e2995e253428cc247ba8ca14bc9);
    /// XMI identity: `Core-Types-Type-intersectingType`.
    pub const TYPE_INTERSECTING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xb03ea33e0db2530faa04e63687a2b845);
    /// XMI identity: `Core-Types-Type-isAbstract`.
    pub const TYPE_IS_ABSTRACT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xbecaa80624f95950828b8057f04f8d42);
    /// XMI identity: `Core-Types-Type-isConjugated`.
    pub const TYPE_IS_CONJUGATED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8c68e15f1d765933a837cd649bae2d64);
    /// XMI identity: `Core-Types-Type-isSufficient`.
    pub const TYPE_IS_SUFFICIENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf5f2b420546554b5b7b78ee89f76265e);
    /// XMI identity: `Core-Types-Type-multiplicity`.
    pub const TYPE_MULTIPLICITY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x663791763da1551381bddc026ae92a56);
    /// XMI identity: `Core-Types-Type-output`.
    pub const TYPE_OUTPUT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x62b911b7a7b9540c8b0d1ada0f554e12);
    /// XMI identity: `Core-Types-Type-ownedConjugator`.
    pub const TYPE_OWNED_CONJUGATOR: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfe053bd23e57553d8a160f83175a7779);
    /// XMI identity: `Core-Types-Type-ownedDifferencing`.
    pub const TYPE_OWNED_DIFFERENCING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6dde63ad31f65075a24bdd24cc21fb0a);
    /// XMI identity: `Core-Types-Type-ownedDisjoining`.
    pub const TYPE_OWNED_DISJOINING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x17943ca9739158bdbf968992b0cddb0a);
    /// XMI identity: `Core-Types-Type-ownedEndFeature`.
    pub const TYPE_OWNED_END_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x56214dae21f75e28af817cd42c6b40a0);
    /// XMI identity: `Core-Types-Type-ownedFeature`.
    pub const TYPE_OWNED_FEATURE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd1b31d871c2f5e4ea0b0aa944feb6275);
    /// XMI identity: `Core-Types-Type-ownedFeatureMembership`.
    pub const TYPE_OWNED_FEATURE_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x3dafabbbb89c5eea9d701cd4f6f83e32);
    /// XMI identity: `Core-Types-Type-ownedIntersecting`.
    pub const TYPE_OWNED_INTERSECTING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xaae025885f9551fba822b5ee1b633597);
    /// XMI identity: `Core-Types-Type-ownedSpecialization`.
    pub const TYPE_OWNED_SPECIALIZATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x2e310f4cfcfa592c88f7fd6e2d91bf73);
    /// XMI identity: `Core-Types-Type-ownedUnioning`.
    pub const TYPE_OWNED_UNIONING: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc5d536f2ca245717ba40dc863ba89284);
    /// XMI identity: `Core-Types-Type-unioningType`.
    pub const TYPE_UNIONING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1ac4247b1b5156f89d628985ffba367a);
    /// XMI identity: `Core-Types-Unioning-typeUnioned`.
    pub const UNIONING_TYPE_UNIONED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6c7514a098085805b4981aa7aa15c0ac);
    /// XMI identity: `Core-Types-Unioning-unioningType`.
    pub const UNIONING_UNIONING_TYPE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x180d23b23ceb58d785175c33f1450a6c);
    /// XMI identity: `Root-Annotations-A_annotatedElement_annotatingElement-annotatingElement`.
    pub const A_ANNOTATED_ELEMENT_ANNOTATING_ELEMENT_ANNOTATING_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x99666e9cc81656f2913f0e27fd2815a4);
    /// XMI identity: `Root-Annotations-A_annotatedElement_annotation-annotation`.
    pub const A_ANNOTATED_ELEMENT_ANNOTATION_ANNOTATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x75d29c07e670506d9ce6ccb75b138064);
    /// XMI identity: `Root-Annotations-AnnotatingElement-annotatedElement`.
    pub const ANNOTATING_ELEMENT_ANNOTATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfda6ad645a1951738debc5d180ceb887);
    /// XMI identity: `Root-Annotations-AnnotatingElement-annotation`.
    pub const ANNOTATING_ELEMENT_ANNOTATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf1b39d3c1e32577fb0ad65f4d2dc4ec9);
    /// XMI identity: `Root-Annotations-AnnotatingElement-ownedAnnotatingRelationship`.
    pub const ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa9d44604827650f49978cb06e4052c1f);
    /// XMI identity: `Root-Annotations-AnnotatingElement-owningAnnotatingRelationship`.
    pub const ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x584cc0f0eeb75024abe24b556c1d4054);
    /// XMI identity: `Root-Annotations-Annotation-annotatedElement`.
    pub const ANNOTATION_ANNOTATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x19119ba7bf1c5b17997656a77c034412);
    /// XMI identity: `Root-Annotations-Annotation-annotatingElement`.
    pub const ANNOTATION_ANNOTATING_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa12e92f622fb5c43965d64e3e3cfff7c);
    /// XMI identity: `Root-Annotations-Annotation-ownedAnnotatingElement`.
    pub const ANNOTATION_OWNED_ANNOTATING_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xba888040c0705a9681bfe8e607ecd694);
    /// XMI identity: `Root-Annotations-Annotation-owningAnnotatedElement`.
    pub const ANNOTATION_OWNING_ANNOTATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x21318515c31055aebf6f8777b3c520eb);
    /// XMI identity: `Root-Annotations-Annotation-owningAnnotatingElement`.
    pub const ANNOTATION_OWNING_ANNOTATING_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x338bd661f7225d81bebc402ab84e18e1);
    /// XMI identity: `Root-Annotations-Comment-body`.
    pub const COMMENT_BODY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xe12d87fcbde452898656f6b715244ad4);
    /// XMI identity: `Root-Annotations-Comment-locale`.
    pub const COMMENT_LOCALE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xccdb6665746854adbe1fd6c4232a8b6e);
    /// XMI identity: `Root-Annotations-Documentation-documentedElement`.
    pub const DOCUMENTATION_DOCUMENTED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x8cd198057dae5fd1bdeebf94ffda6ced);
    /// XMI identity: `Root-Annotations-TextualRepresentation-body`.
    pub const TEXTUAL_REPRESENTATION_BODY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x191d06ce4eaf5b9aaf53463dddb245a4);
    /// XMI identity: `Root-Annotations-TextualRepresentation-language`.
    pub const TEXTUAL_REPRESENTATION_LANGUAGE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x5f976b79b7d856afa959575e63088358);
    /// XMI identity: `Root-Annotations-TextualRepresentation-representedElement`.
    pub const TEXTUAL_REPRESENTATION_REPRESENTED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x2d1948f8604e53f9810cbf3ca5faadb8);
    /// XMI identity: `Root-Elements-A_relatedElement_relationship-relationship`.
    pub const A_RELATED_ELEMENT_RELATIONSHIP_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf3809b145da0526f8437916e0ca905bf);
    /// XMI identity: `Root-Elements-A_source_sourceRelationship-sourceRelationship`.
    pub const A_SOURCE_SOURCE_RELATIONSHIP_SOURCE_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7c9425f3d08457bda98e461b3c8f41ee);
    /// XMI identity: `Root-Elements-A_target_targetRelationship-targetRelationship`.
    pub const A_TARGET_TARGET_RELATIONSHIP_TARGET_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x1066516fca9f5a9e8dfb3c84969fb5f4);
    /// XMI identity: `Root-Elements-Element-aliasIds`.
    pub const ELEMENT_ALIAS_IDS: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x972791cf3961509a9cd39ed91b0bd34d);
    /// XMI identity: `Root-Elements-Element-declaredName`.
    pub const ELEMENT_DECLARED_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xcb8a41585bc45dcea20836825c32b290);
    /// XMI identity: `Root-Elements-Element-declaredShortName`.
    pub const ELEMENT_DECLARED_SHORT_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x706bed819d935bb6911bd3a9a4a334c8);
    /// XMI identity: `Root-Elements-Element-documentation`.
    pub const ELEMENT_DOCUMENTATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xaa312aef5b5c592f81c3cdf50e81bd11);
    /// XMI identity: `Root-Elements-Element-elementId`.
    pub const ELEMENT_ELEMENT_ID: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xf7791531eb7e5db980b7a525d4c75ab2);
    /// XMI identity: `Root-Elements-Element-isImpliedIncluded`.
    pub const ELEMENT_IS_IMPLIED_INCLUDED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7ce0f2ea7d4b5956a5f6e1d15175d417);
    /// XMI identity: `Root-Elements-Element-isLibraryElement`.
    pub const ELEMENT_IS_LIBRARY_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6882d0c5a98e5d5f8260fe037ac38e9b);
    /// XMI identity: `Root-Elements-Element-name`.
    pub const ELEMENT_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x74e33b8a83a95469a10e0bdd2a22cc3d);
    /// XMI identity: `Root-Elements-Element-ownedAnnotation`.
    pub const ELEMENT_OWNED_ANNOTATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x55660048987757dbaaf9adf7af662189);
    /// XMI identity: `Root-Elements-Element-ownedElement`.
    pub const ELEMENT_OWNED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x77f3c6dcbe425d6486b8954a62f619b3);
    /// XMI identity: `Root-Elements-Element-ownedRelationship`.
    pub const ELEMENT_OWNED_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7ded3a2ac727565699a8b0af6d076ae1);
    /// XMI identity: `Root-Elements-Element-owner`.
    pub const ELEMENT_OWNER: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x74160a4c949b5b42bf7c0b09f6407fb4);
    /// XMI identity: `Root-Elements-Element-owningMembership`.
    pub const ELEMENT_OWNING_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x51dd5d61f71351f8a985d0012c9ee9f6);
    /// XMI identity: `Root-Elements-Element-owningNamespace`.
    pub const ELEMENT_OWNING_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x0676f6ae5ef95ea69f31614566ae90d2);
    /// XMI identity: `Root-Elements-Element-owningRelationship`.
    pub const ELEMENT_OWNING_RELATIONSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x07af25a73496539fb28cbc1d8392419b);
    /// XMI identity: `Root-Elements-Element-qualifiedName`.
    pub const ELEMENT_QUALIFIED_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfeddcf75e6535c8893384905313a1933);
    /// XMI identity: `Root-Elements-Element-shortName`.
    pub const ELEMENT_SHORT_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xe519265e0ca2564c80f9928ba6668bbd);
    /// XMI identity: `Root-Elements-Element-textualRepresentation`.
    pub const ELEMENT_TEXTUAL_REPRESENTATION: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd7b2e29ae7b956c1b94c83af651e0496);
    /// XMI identity: `Root-Elements-Relationship-isImplied`.
    pub const RELATIONSHIP_IS_IMPLIED: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x3297ccb386295962aaab2b7d6ea829d7);
    /// XMI identity: `Root-Elements-Relationship-ownedRelatedElement`.
    pub const RELATIONSHIP_OWNED_RELATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7d4b38790f73512e976d67141ffcadba);
    /// XMI identity: `Root-Elements-Relationship-owningRelatedElement`.
    pub const RELATIONSHIP_OWNING_RELATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfb8a39fe9bc25979b7aa2f5020bf6a7d);
    /// XMI identity: `Root-Elements-Relationship-relatedElement`.
    pub const RELATIONSHIP_RELATED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xab76de306ba1530c81e48b151b68df9a);
    /// XMI identity: `Root-Elements-Relationship-source`.
    pub const RELATIONSHIP_SOURCE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xa4780cc15a015f7f823a7e66916f79da);
    /// XMI identity: `Root-Elements-Relationship-target`.
    pub const RELATIONSHIP_TARGET: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x77b56cbb8a695fdfb3362420d3f95a80);
    /// XMI identity: `Root-Namespaces-A_importedElement_membershipImport-membershipImport`.
    pub const A_IMPORTED_ELEMENT_MEMBERSHIP_IMPORT_MEMBERSHIP_IMPORT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd366625e69295147872a78a4c506a5f6);
    /// XMI identity: `Root-Namespaces-A_importedMembership_importingNamespace-importingNamespace`.
    pub const A_IMPORTED_MEMBERSHIP_IMPORTING_NAMESPACE_IMPORTING_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x004b5602f57c5ba18ee69ffa8adb07ad);
    /// XMI identity: `Root-Namespaces-A_memberElement_membership-membership`.
    pub const A_MEMBER_ELEMENT_MEMBERSHIP_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xc7ef9ca14e4d5ca0ab103df667b0b17f);
    /// XMI identity: `Root-Namespaces-A_member_namespace-namespace`.
    pub const A_MEMBER_NAMESPACE_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xec9d7042a17a50c9a7300dd8cf4dc790);
    /// XMI identity: `Root-Namespaces-A_membership_membershipNamespace-membershipNamespace`.
    pub const A_MEMBERSHIP_MEMBERSHIP_NAMESPACE_MEMBERSHIP_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfa744d680b5a5e3ab5bf3efcb055ac34);
    /// XMI identity: `Root-Namespaces-Import-importOwningNamespace`.
    pub const IMPORT_IMPORT_OWNING_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd6e30cb01ed8523aba900b57ed7f70d1);
    /// XMI identity: `Root-Namespaces-Import-importedElement`.
    pub const IMPORT_IMPORTED_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x514459b74e1155cbab213cacac02942c);
    /// XMI identity: `Root-Namespaces-Import-isImportAll`.
    pub const IMPORT_IS_IMPORT_ALL: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xcbc6737d4100558b987961899dcffb1f);
    /// XMI identity: `Root-Namespaces-Import-isRecursive`.
    pub const IMPORT_IS_RECURSIVE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x74b0980b28a05b0fabdaf5aac641c2f0);
    /// XMI identity: `Root-Namespaces-Import-visibility`.
    pub const IMPORT_VISIBILITY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x92428b7ec7c45578ad0df5470a7d2aa4);
    /// XMI identity: `Root-Namespaces-Membership-memberElement`.
    pub const MEMBERSHIP_MEMBER_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x87f10741a872520cbf55286fddc10bb8);
    /// XMI identity: `Root-Namespaces-Membership-memberElementId`.
    pub const MEMBERSHIP_MEMBER_ELEMENT_ID: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xd6f5eac8c702518cae52185f31295df5);
    /// XMI identity: `Root-Namespaces-Membership-memberName`.
    pub const MEMBERSHIP_MEMBER_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7b249527a2c6555898319656a3fc6a9a);
    /// XMI identity: `Root-Namespaces-Membership-memberShortName`.
    pub const MEMBERSHIP_MEMBER_SHORT_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x3dc5377df7fb592cb6c63d7dd701dc9b);
    /// XMI identity: `Root-Namespaces-Membership-membershipOwningNamespace`.
    pub const MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x19e41ddb05b45211b8b792a3e6d8683f);
    /// XMI identity: `Root-Namespaces-Membership-visibility`.
    pub const MEMBERSHIP_VISIBILITY: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x002548b0653b546aabb65b5c21359a5e);
    /// XMI identity: `Root-Namespaces-Namespace-importedMembership`.
    pub const NAMESPACE_IMPORTED_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x59a7f21818d55634aac57e17f1a5c4d9);
    /// XMI identity: `Root-Namespaces-Namespace-member`.
    pub const NAMESPACE_MEMBER: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x79219bed65e158f1b6094016c1917b09);
    /// XMI identity: `Root-Namespaces-Namespace-membership`.
    pub const NAMESPACE_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x6a53cbc23865570cb4833782f0f77a0b);
    /// XMI identity: `Root-Namespaces-Namespace-ownedImport`.
    pub const NAMESPACE_OWNED_IMPORT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7d8746ea4f5e51378a14f93beab0d09b);
    /// XMI identity: `Root-Namespaces-Namespace-ownedMember`.
    pub const NAMESPACE_OWNED_MEMBER: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x829328963b8b504791e9da26b32021a9);
    /// XMI identity: `Root-Namespaces-Namespace-ownedMembership`.
    pub const NAMESPACE_OWNED_MEMBERSHIP: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x7953335db80f5a768946b4ee421ab41e);
    /// XMI identity: `Root-Namespaces-OwningMembership-ownedMemberElement`.
    pub const OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x580c31a16d4d56bd983fe14594951d58);
    /// XMI identity: `Root-Namespaces-OwningMembership-ownedMemberElementId`.
    pub const OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x919ea97b49655a6b9e4e736b4421211d);
    /// XMI identity: `Root-Namespaces-OwningMembership-ownedMemberName`.
    pub const OWNING_MEMBERSHIP_OWNED_MEMBER_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x4c85e74e399e5372bd9c2be6796bfd47);
    /// XMI identity: `Root-Namespaces-OwningMembership-ownedMemberShortName`.
    pub const OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0xfe41d7a6de5c5b3e95e0b841dcd5f78e);
}

/// Borrowed views for exactly the registered Root/Core closure.
#[rustfmt::skip]
pub mod views {
    use super::{classes, properties};
    use agq_kernel::{ElementId, EnumerationLiteralId, EnumerationId, metamodel::ValueKind};
    use crate::{Values, ViewError, view::{define_view, read}};

    define_view!(CrossSubsetting, classes::CROSS_SUBSETTING);
    impl<'m> CrossSubsetting<'m> {
        /// View the same record through its normative supertype.
        pub fn as_subsetting(self) -> Result<Subsetting<'m>, ViewError> { Subsetting::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_specialization(self) -> Result<Specialization<'m>, ViewError> { Specialization::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::CROSS_SUBSETTING_CROSSED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn crossed_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::CROSS_SUBSETTING_CROSSED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::CROSS_SUBSETTING_CROSSING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn crossing_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::CROSS_SUBSETTING_CROSSING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Feature, classes::FEATURE);
    impl<'m> Feature<'m> {
        /// View the same record through its normative supertype.
        pub fn as_type(self) -> Result<Type<'m>, ViewError> { Type::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_namespace(self) -> Result<Namespace<'m>, ViewError> { Namespace::try_new(self.id(), self.model()) }
        /// Read [`properties::FEATURE_IS_END`]; resolves effective redefinitions by identity.
        pub fn is_end(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_END, ValueKind::Boolean) }
        /// Read [`properties::TYPE_INHERITED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn inherited_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_IS_ORDERED`]; resolves effective redefinitions by identity.
        pub fn is_ordered(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_ORDERED, ValueKind::Boolean) }
        /// Read [`properties::TYPE_DIFFERENCING_TYPE`]; resolves effective redefinitions by identity.
        pub fn differencing_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIFFERENCING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_OWNED_DISJOINING`]; resolves effective redefinitions by identity.
        pub fn owned_disjoining(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DISJOINING, ValueKind::Reference(classes::DISJOINING)) }
        /// Read [`properties::TYPE_UNIONING_TYPE`]; resolves effective redefinitions by identity.
        pub fn unioning_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_UNIONING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_IS_VARIABLE`]; resolves effective redefinitions by identity.
        pub fn is_variable(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_VARIABLE, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_OWNED_CROSS_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_cross_subsetting(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNED_CROSS_SUBSETTING, ValueKind::Reference(classes::CROSS_SUBSETTING)) }
        /// Read [`properties::TYPE_OWNED_SPECIALIZATION`]; resolves effective redefinitions by identity.
        pub fn owned_specialization(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_SPECIALIZATION, ValueKind::Reference(classes::SPECIALIZATION)) }
        /// Read [`properties::FEATURE_OWNED_FEATURE_CHAINING`]; resolves effective redefinitions by identity.
        pub fn owned_feature_chaining(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_FEATURE_CHAINING, ValueKind::Reference(classes::FEATURE_CHAINING)) }
        /// Read [`properties::FEATURE_OWNED_TYPING`]; resolves effective redefinitions by identity.
        pub fn owned_typing(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_TYPING, ValueKind::Reference(classes::FEATURE_TYPING)) }
        /// Read [`properties::FEATURE_OWNED_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_subsetting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_SUBSETTING, ValueKind::Reference(classes::SUBSETTING)) }
        /// Read [`properties::FEATURE_IS_UNIQUE`]; resolves effective redefinitions by identity.
        pub fn is_unique(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_UNIQUE, ValueKind::Boolean) }
        /// Read [`properties::TYPE_OWNED_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::FEATURE_IS_DERIVED`]; resolves effective redefinitions by identity.
        pub fn is_derived(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_DERIVED, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_TYPE`]; resolves effective redefinitions by identity.
        pub fn r#type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_OWNED_FEATURE_INVERTING`]; resolves effective redefinitions by identity.
        pub fn owned_feature_inverting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_FEATURE_INVERTING, ValueKind::Reference(classes::FEATURE_INVERTING)) }
        /// Read [`properties::FEATURE_FEATURING_TYPE`]; resolves effective redefinitions by identity.
        pub fn featuring_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_FEATURING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::TYPE_OWNED_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_IMPORTED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn imported_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_IMPORTED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::FEATURE_IS_PORTION`]; resolves effective redefinitions by identity.
        pub fn is_portion(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_PORTION, ValueKind::Boolean) }
        /// Read [`properties::TYPE_FEATURE`]; resolves effective redefinitions by identity.
        pub fn feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OUTPUT`]; resolves effective redefinitions by identity.
        pub fn output(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OUTPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_MULTIPLICITY`]; resolves effective redefinitions by identity.
        pub fn multiplicity(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_MULTIPLICITY, ValueKind::Reference(classes::MULTIPLICITY)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::TYPE_INHERITED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn inherited_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::NAMESPACE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_DIFFERENCING`]; resolves effective redefinitions by identity.
        pub fn owned_differencing(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DIFFERENCING, ValueKind::Reference(classes::DIFFERENCING)) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_MEMBER`]; resolves effective redefinitions by identity.
        pub fn member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::FEATURE_IS_CONSTANT`]; resolves effective redefinitions by identity.
        pub fn is_constant(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_CONSTANT, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_IMPORT`]; resolves effective redefinitions by identity.
        pub fn owned_import(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_IMPORT, ValueKind::Reference(classes::IMPORT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_INPUT`]; resolves effective redefinitions by identity.
        pub fn input(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBER`]; resolves effective redefinitions by identity.
        pub fn owned_member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::TYPE_IS_CONJUGATED`]; resolves effective redefinitions by identity.
        pub fn is_conjugated(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_CONJUGATED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::FEATURE_OWNED_TYPE_FEATURING`]; resolves effective redefinitions by identity.
        pub fn owned_type_featuring(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_TYPE_FEATURING, ValueKind::Reference(classes::TYPE_FEATURING)) }
        /// Read [`properties::FEATURE_DIRECTION`]; resolves effective redefinitions by identity.
        pub fn direction(self) -> Result<Option<EnumerationLiteralId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_DIRECTION, ValueKind::Enumeration(EnumerationId::from_u128(0x244d70d8d62b572a8edfbda07e2c1866))) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::TYPE_OWNED_INTERSECTING`]; resolves effective redefinitions by identity.
        pub fn owned_intersecting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_INTERSECTING, ValueKind::Reference(classes::INTERSECTING)) }
        /// Read [`properties::TYPE_INTERSECTING_TYPE`]; resolves effective redefinitions by identity.
        pub fn intersecting_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INTERSECTING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::TYPE_DIRECTED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn directed_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIRECTED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_IS_ABSTRACT`]; resolves effective redefinitions by identity.
        pub fn is_abstract(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_ABSTRACT, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_CHAINING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn chaining_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_CHAINING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OWNED_UNIONING`]; resolves effective redefinitions by identity.
        pub fn owned_unioning(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_UNIONING, ValueKind::Reference(classes::UNIONING)) }
        /// Read [`properties::FEATURE_OWNED_REDEFINITION`]; resolves effective redefinitions by identity.
        pub fn owned_redefinition(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_REDEFINITION, ValueKind::Reference(classes::REDEFINITION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::FEATURE_OWNED_REFERENCE_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_reference_subsetting(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNED_REFERENCE_SUBSETTING, ValueKind::Reference(classes::REFERENCE_SUBSETTING)) }
        /// Read [`properties::TYPE_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_OWNING_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_feature_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNING_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::FEATURE_FEATURE_TARGET`]; resolves effective redefinitions by identity.
        pub fn feature_target(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_FEATURE_TARGET, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_END_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn end_owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_END_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::FEATURE_IS_COMPOSITE`]; resolves effective redefinitions by identity.
        pub fn is_composite(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_COMPOSITE, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_CROSS_FEATURE`]; resolves effective redefinitions by identity.
        pub fn cross_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_CROSS_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_IS_SUFFICIENT`]; resolves effective redefinitions by identity.
        pub fn is_sufficient(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_SUFFICIENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::TYPE_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OWNED_CONJUGATOR`]; resolves effective redefinitions by identity.
        pub fn owned_conjugator(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_OWNED_CONJUGATOR, ValueKind::Reference(classes::CONJUGATION)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(FeatureChaining, classes::FEATURE_CHAINING);
    impl<'m> FeatureChaining<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::FEATURE_CHAINING_FEATURE_CHAINED`]; resolves effective redefinitions by identity.
        pub fn feature_chained(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_CHAINING_FEATURE_CHAINED, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::FEATURE_CHAINING_CHAINING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn chaining_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_CHAINING_CHAINING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(FeatureInverting, classes::FEATURE_INVERTING);
    impl<'m> FeatureInverting<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::FEATURE_INVERTING_OWNING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owning_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_INVERTING_OWNING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::FEATURE_INVERTING_FEATURE_INVERTED`]; resolves effective redefinitions by identity.
        pub fn feature_inverted(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_INVERTING_FEATURE_INVERTED, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_INVERTING_INVERTING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn inverting_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_INVERTING_INVERTING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(FeatureTyping, classes::FEATURE_TYPING);
    impl<'m> FeatureTyping<'m> {
        /// View the same record through its normative supertype.
        pub fn as_specialization(self) -> Result<Specialization<'m>, ViewError> { Specialization::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::FEATURE_TYPING_TYPE`]; resolves effective redefinitions by identity.
        pub fn r#type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_TYPING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_TYPING_OWNING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owning_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_TYPING_OWNING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::FEATURE_TYPING_TYPED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn typed_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_TYPING_TYPED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Redefinition, classes::REDEFINITION);
    impl<'m> Redefinition<'m> {
        /// View the same record through its normative supertype.
        pub fn as_subsetting(self) -> Result<Subsetting<'m>, ViewError> { Subsetting::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_specialization(self) -> Result<Specialization<'m>, ViewError> { Specialization::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::REDEFINITION_REDEFINING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn redefining_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::REDEFINITION_REDEFINING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::SUBSETTING_OWNING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owning_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::SUBSETTING_OWNING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::REDEFINITION_REDEFINED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn redefined_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::REDEFINITION_REDEFINED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(ReferenceSubsetting, classes::REFERENCE_SUBSETTING);
    impl<'m> ReferenceSubsetting<'m> {
        /// View the same record through its normative supertype.
        pub fn as_subsetting(self) -> Result<Subsetting<'m>, ViewError> { Subsetting::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_specialization(self) -> Result<Specialization<'m>, ViewError> { Specialization::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::REFERENCE_SUBSETTING_REFERENCING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn referencing_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::REFERENCE_SUBSETTING_REFERENCING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::REFERENCE_SUBSETTING_REFERENCED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn referenced_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::REFERENCE_SUBSETTING_REFERENCED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Subsetting, classes::SUBSETTING);
    impl<'m> Subsetting<'m> {
        /// View the same record through its normative supertype.
        pub fn as_specialization(self) -> Result<Specialization<'m>, ViewError> { Specialization::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::SUBSETTING_OWNING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owning_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::SUBSETTING_OWNING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::SUBSETTING_SUBSETTING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn subsetting_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::SUBSETTING_SUBSETTING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::SUBSETTING_SUBSETTED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn subsetted_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::SUBSETTING_SUBSETTED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(TypeFeaturing, classes::TYPE_FEATURING);
    impl<'m> TypeFeaturing<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::TYPE_FEATURING_OWNING_FEATURE_OF_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_feature_of_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_FEATURING_OWNING_FEATURE_OF_TYPE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::TYPE_FEATURING_FEATURE_OF_TYPE`]; resolves effective redefinitions by identity.
        pub fn feature_of_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::TYPE_FEATURING_FEATURE_OF_TYPE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::TYPE_FEATURING_FEATURING_TYPE`]; resolves effective redefinitions by identity.
        pub fn featuring_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::TYPE_FEATURING_FEATURING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Conjugation, classes::CONJUGATION);
    impl<'m> Conjugation<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::CONJUGATION_CONJUGATED_TYPE`]; resolves effective redefinitions by identity.
        pub fn conjugated_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::CONJUGATION_CONJUGATED_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::CONJUGATION_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::CONJUGATION_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::CONJUGATION_ORIGINAL_TYPE`]; resolves effective redefinitions by identity.
        pub fn original_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::CONJUGATION_ORIGINAL_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Differencing, classes::DIFFERENCING);
    impl<'m> Differencing<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::DIFFERENCING_TYPE_DIFFERENCED`]; resolves effective redefinitions by identity.
        pub fn type_differenced(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::DIFFERENCING_TYPE_DIFFERENCED, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::DIFFERENCING_DIFFERENCING_TYPE`]; resolves effective redefinitions by identity.
        pub fn differencing_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::DIFFERENCING_DIFFERENCING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Disjoining, classes::DISJOINING);
    impl<'m> Disjoining<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::DISJOINING_DISJOINING_TYPE`]; resolves effective redefinitions by identity.
        pub fn disjoining_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::DISJOINING_DISJOINING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::DISJOINING_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::DISJOINING_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::DISJOINING_TYPE_DISJOINED`]; resolves effective redefinitions by identity.
        pub fn type_disjoined(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::DISJOINING_TYPE_DISJOINED, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(FeatureMembership, classes::FEATURE_MEMBERSHIP);
    impl<'m> FeatureMembership<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_membership(self) -> Result<Membership<'m>, ViewError> { Membership::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_owning_membership(self) -> Result<OwningMembership<'m>, ViewError> { OwningMembership::try_new(self.id(), self.model()) }
        /// Read [`properties::MEMBERSHIP_VISIBILITY`]; resolves effective redefinitions by identity.
        pub fn visibility(self) -> Result<EnumerationLiteralId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_VISIBILITY, ValueKind::Enumeration(EnumerationId::from_u128(0x786f4ae9d793543c9455f0f6485331f2))) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_NAME`]; resolves effective redefinitions by identity.
        pub fn owned_member_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_member_feature(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn owned_member_element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::FEATURE_MEMBERSHIP_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_MEMBERSHIP_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn owned_member_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Intersecting, classes::INTERSECTING);
    impl<'m> Intersecting<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::INTERSECTING_TYPE_INTERSECTED`]; resolves effective redefinitions by identity.
        pub fn type_intersected(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::INTERSECTING_TYPE_INTERSECTED, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::INTERSECTING_INTERSECTING_TYPE`]; resolves effective redefinitions by identity.
        pub fn intersecting_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::INTERSECTING_INTERSECTING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Multiplicity, classes::MULTIPLICITY);
    impl<'m> Multiplicity<'m> {
        /// View the same record through its normative supertype.
        pub fn as_feature(self) -> Result<Feature<'m>, ViewError> { Feature::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_type(self) -> Result<Type<'m>, ViewError> { Type::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_namespace(self) -> Result<Namespace<'m>, ViewError> { Namespace::try_new(self.id(), self.model()) }
        /// Read [`properties::FEATURE_IS_END`]; resolves effective redefinitions by identity.
        pub fn is_end(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_END, ValueKind::Boolean) }
        /// Read [`properties::TYPE_INHERITED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn inherited_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_IS_ORDERED`]; resolves effective redefinitions by identity.
        pub fn is_ordered(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_ORDERED, ValueKind::Boolean) }
        /// Read [`properties::TYPE_DIFFERENCING_TYPE`]; resolves effective redefinitions by identity.
        pub fn differencing_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIFFERENCING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_OWNED_DISJOINING`]; resolves effective redefinitions by identity.
        pub fn owned_disjoining(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DISJOINING, ValueKind::Reference(classes::DISJOINING)) }
        /// Read [`properties::TYPE_UNIONING_TYPE`]; resolves effective redefinitions by identity.
        pub fn unioning_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_UNIONING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_IS_VARIABLE`]; resolves effective redefinitions by identity.
        pub fn is_variable(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_VARIABLE, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_OWNED_CROSS_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_cross_subsetting(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNED_CROSS_SUBSETTING, ValueKind::Reference(classes::CROSS_SUBSETTING)) }
        /// Read [`properties::TYPE_OWNED_SPECIALIZATION`]; resolves effective redefinitions by identity.
        pub fn owned_specialization(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_SPECIALIZATION, ValueKind::Reference(classes::SPECIALIZATION)) }
        /// Read [`properties::FEATURE_OWNED_FEATURE_CHAINING`]; resolves effective redefinitions by identity.
        pub fn owned_feature_chaining(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_FEATURE_CHAINING, ValueKind::Reference(classes::FEATURE_CHAINING)) }
        /// Read [`properties::FEATURE_OWNED_TYPING`]; resolves effective redefinitions by identity.
        pub fn owned_typing(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_TYPING, ValueKind::Reference(classes::FEATURE_TYPING)) }
        /// Read [`properties::FEATURE_OWNED_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_subsetting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_SUBSETTING, ValueKind::Reference(classes::SUBSETTING)) }
        /// Read [`properties::FEATURE_IS_UNIQUE`]; resolves effective redefinitions by identity.
        pub fn is_unique(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_UNIQUE, ValueKind::Boolean) }
        /// Read [`properties::TYPE_OWNED_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::FEATURE_IS_DERIVED`]; resolves effective redefinitions by identity.
        pub fn is_derived(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_DERIVED, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_TYPE`]; resolves effective redefinitions by identity.
        pub fn r#type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::FEATURE_OWNED_FEATURE_INVERTING`]; resolves effective redefinitions by identity.
        pub fn owned_feature_inverting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_FEATURE_INVERTING, ValueKind::Reference(classes::FEATURE_INVERTING)) }
        /// Read [`properties::FEATURE_FEATURING_TYPE`]; resolves effective redefinitions by identity.
        pub fn featuring_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_FEATURING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::TYPE_OWNED_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_IMPORTED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn imported_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_IMPORTED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::FEATURE_IS_PORTION`]; resolves effective redefinitions by identity.
        pub fn is_portion(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_PORTION, ValueKind::Boolean) }
        /// Read [`properties::TYPE_FEATURE`]; resolves effective redefinitions by identity.
        pub fn feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OUTPUT`]; resolves effective redefinitions by identity.
        pub fn output(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OUTPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_MULTIPLICITY`]; resolves effective redefinitions by identity.
        pub fn multiplicity(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_MULTIPLICITY, ValueKind::Reference(classes::MULTIPLICITY)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::TYPE_INHERITED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn inherited_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::NAMESPACE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_DIFFERENCING`]; resolves effective redefinitions by identity.
        pub fn owned_differencing(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DIFFERENCING, ValueKind::Reference(classes::DIFFERENCING)) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_MEMBER`]; resolves effective redefinitions by identity.
        pub fn member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::FEATURE_IS_CONSTANT`]; resolves effective redefinitions by identity.
        pub fn is_constant(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_CONSTANT, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_IMPORT`]; resolves effective redefinitions by identity.
        pub fn owned_import(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_IMPORT, ValueKind::Reference(classes::IMPORT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_INPUT`]; resolves effective redefinitions by identity.
        pub fn input(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBER`]; resolves effective redefinitions by identity.
        pub fn owned_member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::TYPE_IS_CONJUGATED`]; resolves effective redefinitions by identity.
        pub fn is_conjugated(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_CONJUGATED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::FEATURE_OWNED_TYPE_FEATURING`]; resolves effective redefinitions by identity.
        pub fn owned_type_featuring(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_TYPE_FEATURING, ValueKind::Reference(classes::TYPE_FEATURING)) }
        /// Read [`properties::FEATURE_DIRECTION`]; resolves effective redefinitions by identity.
        pub fn direction(self) -> Result<Option<EnumerationLiteralId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_DIRECTION, ValueKind::Enumeration(EnumerationId::from_u128(0x244d70d8d62b572a8edfbda07e2c1866))) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::TYPE_OWNED_INTERSECTING`]; resolves effective redefinitions by identity.
        pub fn owned_intersecting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_INTERSECTING, ValueKind::Reference(classes::INTERSECTING)) }
        /// Read [`properties::TYPE_INTERSECTING_TYPE`]; resolves effective redefinitions by identity.
        pub fn intersecting_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INTERSECTING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::TYPE_DIRECTED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn directed_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIRECTED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_IS_ABSTRACT`]; resolves effective redefinitions by identity.
        pub fn is_abstract(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_ABSTRACT, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_CHAINING_FEATURE`]; resolves effective redefinitions by identity.
        pub fn chaining_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_CHAINING_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OWNED_UNIONING`]; resolves effective redefinitions by identity.
        pub fn owned_unioning(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_UNIONING, ValueKind::Reference(classes::UNIONING)) }
        /// Read [`properties::FEATURE_OWNED_REDEFINITION`]; resolves effective redefinitions by identity.
        pub fn owned_redefinition(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::FEATURE_OWNED_REDEFINITION, ValueKind::Reference(classes::REDEFINITION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::FEATURE_OWNED_REFERENCE_SUBSETTING`]; resolves effective redefinitions by identity.
        pub fn owned_reference_subsetting(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNED_REFERENCE_SUBSETTING, ValueKind::Reference(classes::REFERENCE_SUBSETTING)) }
        /// Read [`properties::TYPE_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_OWNING_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_feature_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_OWNING_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::FEATURE_FEATURE_TARGET`]; resolves effective redefinitions by identity.
        pub fn feature_target(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_FEATURE_TARGET, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::FEATURE_END_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn end_owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_END_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::FEATURE_IS_COMPOSITE`]; resolves effective redefinitions by identity.
        pub fn is_composite(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::FEATURE_IS_COMPOSITE, ValueKind::Boolean) }
        /// Read [`properties::FEATURE_CROSS_FEATURE`]; resolves effective redefinitions by identity.
        pub fn cross_feature(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::FEATURE_CROSS_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_IS_SUFFICIENT`]; resolves effective redefinitions by identity.
        pub fn is_sufficient(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_SUFFICIENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::TYPE_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OWNED_CONJUGATOR`]; resolves effective redefinitions by identity.
        pub fn owned_conjugator(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_OWNED_CONJUGATOR, ValueKind::Reference(classes::CONJUGATION)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Specialization, classes::SPECIALIZATION);
    impl<'m> Specialization<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::SPECIALIZATION_OWNING_TYPE`]; resolves effective redefinitions by identity.
        pub fn owning_type(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::SPECIALIZATION_OWNING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::SPECIALIZATION_GENERAL`]; resolves effective redefinitions by identity.
        pub fn general(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::SPECIALIZATION_GENERAL, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::SPECIALIZATION_SPECIFIC`]; resolves effective redefinitions by identity.
        pub fn specific(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::SPECIALIZATION_SPECIFIC, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Type, classes::TYPE);
    impl<'m> Type<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_namespace(self) -> Result<Namespace<'m>, ViewError> { Namespace::try_new(self.id(), self.model()) }
        /// Read [`properties::TYPE_INHERITED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn inherited_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_DIFFERENCING_TYPE`]; resolves effective redefinitions by identity.
        pub fn differencing_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIFFERENCING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_OWNED_DISJOINING`]; resolves effective redefinitions by identity.
        pub fn owned_disjoining(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DISJOINING, ValueKind::Reference(classes::DISJOINING)) }
        /// Read [`properties::TYPE_UNIONING_TYPE`]; resolves effective redefinitions by identity.
        pub fn unioning_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_UNIONING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::TYPE_OWNED_SPECIALIZATION`]; resolves effective redefinitions by identity.
        pub fn owned_specialization(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_SPECIALIZATION, ValueKind::Reference(classes::SPECIALIZATION)) }
        /// Read [`properties::TYPE_OWNED_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::TYPE_OWNED_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_IMPORTED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn imported_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_IMPORTED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::TYPE_FEATURE`]; resolves effective redefinitions by identity.
        pub fn feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OUTPUT`]; resolves effective redefinitions by identity.
        pub fn output(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OUTPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_MULTIPLICITY`]; resolves effective redefinitions by identity.
        pub fn multiplicity(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_MULTIPLICITY, ValueKind::Reference(classes::MULTIPLICITY)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::TYPE_INHERITED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn inherited_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INHERITED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::NAMESPACE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_DIFFERENCING`]; resolves effective redefinitions by identity.
        pub fn owned_differencing(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_DIFFERENCING, ValueKind::Reference(classes::DIFFERENCING)) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_MEMBER`]; resolves effective redefinitions by identity.
        pub fn member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_IMPORT`]; resolves effective redefinitions by identity.
        pub fn owned_import(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_IMPORT, ValueKind::Reference(classes::IMPORT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TYPE_INPUT`]; resolves effective redefinitions by identity.
        pub fn input(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INPUT, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBER`]; resolves effective redefinitions by identity.
        pub fn owned_member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::TYPE_IS_CONJUGATED`]; resolves effective redefinitions by identity.
        pub fn is_conjugated(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_CONJUGATED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::TYPE_OWNED_INTERSECTING`]; resolves effective redefinitions by identity.
        pub fn owned_intersecting(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_INTERSECTING, ValueKind::Reference(classes::INTERSECTING)) }
        /// Read [`properties::TYPE_INTERSECTING_TYPE`]; resolves effective redefinitions by identity.
        pub fn intersecting_type(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_INTERSECTING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::TYPE_DIRECTED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn directed_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_DIRECTED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_IS_ABSTRACT`]; resolves effective redefinitions by identity.
        pub fn is_abstract(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_ABSTRACT, ValueKind::Boolean) }
        /// Read [`properties::TYPE_OWNED_UNIONING`]; resolves effective redefinitions by identity.
        pub fn owned_unioning(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_UNIONING, ValueKind::Reference(classes::UNIONING)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::TYPE_FEATURE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn feature_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_FEATURE_MEMBERSHIP, ValueKind::Reference(classes::FEATURE_MEMBERSHIP)) }
        /// Read [`properties::TYPE_OWNED_FEATURE`]; resolves effective redefinitions by identity.
        pub fn owned_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_OWNED_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::TYPE_IS_SUFFICIENT`]; resolves effective redefinitions by identity.
        pub fn is_sufficient(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::TYPE_IS_SUFFICIENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::TYPE_END_FEATURE`]; resolves effective redefinitions by identity.
        pub fn end_feature(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::TYPE_END_FEATURE, ValueKind::Reference(classes::FEATURE)) }
        /// Read [`properties::TYPE_OWNED_CONJUGATOR`]; resolves effective redefinitions by identity.
        pub fn owned_conjugator(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::TYPE_OWNED_CONJUGATOR, ValueKind::Reference(classes::CONJUGATION)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Unioning, classes::UNIONING);
    impl<'m> Unioning<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::UNIONING_UNIONING_TYPE`]; resolves effective redefinitions by identity.
        pub fn unioning_type(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::UNIONING_UNIONING_TYPE, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::UNIONING_TYPE_UNIONED`]; resolves effective redefinitions by identity.
        pub fn type_unioned(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::UNIONING_TYPE_UNIONED, ValueKind::Reference(classes::TYPE)) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(AnnotatingElement, classes::ANNOTATING_ELEMENT);
    impl<'m> AnnotatingElement<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_annotating_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_annotating_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn annotated_element(self) -> Result<Values<'m, ElementId>, ViewError> { read::required_many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Annotation, classes::ANNOTATION);
    impl<'m> Annotation<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ANNOTATION_ANNOTATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn annotated_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::ANNOTATION_ANNOTATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ANNOTATION_OWNING_ANNOTATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_annotated_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATION_OWNING_ANNOTATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ANNOTATION_OWNING_ANNOTATING_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_annotating_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATION_OWNING_ANNOTATING_ELEMENT, ValueKind::Reference(classes::ANNOTATING_ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ANNOTATION_ANNOTATING_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn annotating_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::ANNOTATION_ANNOTATING_ELEMENT, ValueKind::Reference(classes::ANNOTATING_ELEMENT)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ANNOTATION_OWNED_ANNOTATING_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_annotating_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATION_OWNED_ANNOTATING_ELEMENT, ValueKind::Reference(classes::ANNOTATING_ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Comment, classes::COMMENT);
    impl<'m> Comment<'m> {
        /// View the same record through its normative supertype.
        pub fn as_annotating_element(self) -> Result<AnnotatingElement<'m>, ViewError> { AnnotatingElement::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_annotating_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_annotating_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::COMMENT_LOCALE`]; resolves effective redefinitions by identity.
        pub fn locale(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::COMMENT_LOCALE, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::COMMENT_BODY`]; resolves effective redefinitions by identity.
        pub fn body(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::COMMENT_BODY, ValueKind::String) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn annotated_element(self) -> Result<Values<'m, ElementId>, ViewError> { read::required_many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Documentation, classes::DOCUMENTATION);
    impl<'m> Documentation<'m> {
        /// View the same record through its normative supertype.
        pub fn as_annotating_element(self) -> Result<AnnotatingElement<'m>, ViewError> { AnnotatingElement::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_comment(self) -> Result<Comment<'m>, ViewError> { Comment::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_annotating_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::DOCUMENTATION_DOCUMENTED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn documented_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::DOCUMENTATION_DOCUMENTED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_annotating_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::COMMENT_LOCALE`]; resolves effective redefinitions by identity.
        pub fn locale(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::COMMENT_LOCALE, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::COMMENT_BODY`]; resolves effective redefinitions by identity.
        pub fn body(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::COMMENT_BODY, ValueKind::String) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(TextualRepresentation, classes::TEXTUAL_REPRESENTATION);
    impl<'m> TextualRepresentation<'m> {
        /// View the same record through its normative supertype.
        pub fn as_annotating_element(self) -> Result<AnnotatingElement<'m>, ViewError> { AnnotatingElement::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::TEXTUAL_REPRESENTATION_BODY`]; resolves effective redefinitions by identity.
        pub fn body(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::TEXTUAL_REPRESENTATION_BODY, ValueKind::String) }
        /// Read [`properties::TEXTUAL_REPRESENTATION_REPRESENTED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn represented_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::TEXTUAL_REPRESENTATION_REPRESENTED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_annotating_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::TEXTUAL_REPRESENTATION_LANGUAGE`]; resolves effective redefinitions by identity.
        pub fn language(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::TEXTUAL_REPRESENTATION_LANGUAGE, ValueKind::String) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_annotating_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ANNOTATING_ELEMENT_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ANNOTATING_ELEMENT_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Element, classes::ELEMENT);
    impl<'m> Element<'m> {
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Relationship, classes::RELATIONSHIP);
    impl<'m> Relationship<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_TARGET`]; resolves effective redefinitions by identity.
        pub fn target(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_TARGET, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_SOURCE`]; resolves effective redefinitions by identity.
        pub fn source(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_SOURCE, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Import, classes::IMPORT);
    impl<'m> Import<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::IMPORT_IMPORTED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn imported_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::IMPORT_IMPORTED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::IMPORT_IS_RECURSIVE`]; resolves effective redefinitions by identity.
        pub fn is_recursive(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::IMPORT_IS_RECURSIVE, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_TARGET`]; resolves effective redefinitions by identity.
        pub fn target(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_TARGET, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::IMPORT_VISIBILITY`]; resolves effective redefinitions by identity.
        pub fn visibility(self) -> Result<EnumerationLiteralId, ViewError> { read::required(self.id(), self.model(), properties::IMPORT_VISIBILITY, ValueKind::Enumeration(EnumerationId::from_u128(0x786f4ae9d793543c9455f0f6485331f2))) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::IMPORT_IS_IMPORT_ALL`]; resolves effective redefinitions by identity.
        pub fn is_import_all(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::IMPORT_IS_IMPORT_ALL, ValueKind::Boolean) }
        /// Read [`properties::IMPORT_IMPORT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn import_owning_namespace(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::IMPORT_IMPORT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Membership, classes::MEMBERSHIP);
    impl<'m> Membership<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// Read [`properties::MEMBERSHIP_VISIBILITY`]; resolves effective redefinitions by identity.
        pub fn visibility(self) -> Result<EnumerationLiteralId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_VISIBILITY, ValueKind::Enumeration(EnumerationId::from_u128(0x786f4ae9d793543c9455f0f6485331f2))) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn membership_owning_namespace(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::MEMBERSHIP_MEMBER_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn member_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::MEMBERSHIP_MEMBER_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::MEMBERSHIP_MEMBER_NAME`]; resolves effective redefinitions by identity.
        pub fn member_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::MEMBERSHIP_MEMBER_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::MEMBERSHIP_MEMBER_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn member_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_MEMBER_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::MEMBERSHIP_MEMBER_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn member_element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_MEMBER_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(Namespace, classes::NAMESPACE);
    impl<'m> Namespace<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::NAMESPACE_IMPORTED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn imported_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_IMPORTED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_MEMBER`]; resolves effective redefinitions by identity.
        pub fn member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_membership(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBERSHIP, ValueKind::Reference(classes::MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::NAMESPACE_OWNED_IMPORT`]; resolves effective redefinitions by identity.
        pub fn owned_import(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_IMPORT, ValueKind::Reference(classes::IMPORT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::NAMESPACE_OWNED_MEMBER`]; resolves effective redefinitions by identity.
        pub fn owned_member(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::NAMESPACE_OWNED_MEMBER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }

    define_view!(OwningMembership, classes::OWNING_MEMBERSHIP);
    impl<'m> OwningMembership<'m> {
        /// View the same record through its normative supertype.
        pub fn as_element(self) -> Result<Element<'m>, ViewError> { Element::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_relationship(self) -> Result<Relationship<'m>, ViewError> { Relationship::try_new(self.id(), self.model()) }
        /// View the same record through its normative supertype.
        pub fn as_membership(self) -> Result<Membership<'m>, ViewError> { Membership::try_new(self.id(), self.model()) }
        /// Read [`properties::MEMBERSHIP_VISIBILITY`]; resolves effective redefinitions by identity.
        pub fn visibility(self) -> Result<EnumerationLiteralId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_VISIBILITY, ValueKind::Enumeration(EnumerationId::from_u128(0x786f4ae9d793543c9455f0f6485331f2))) }
        /// Read [`properties::ELEMENT_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn owning_namespace(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::ELEMENT_OWNING_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_relationship(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE`]; resolves effective redefinitions by identity.
        pub fn membership_owning_namespace(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE, ValueKind::Reference(classes::NAMESPACE)) }
        /// Read [`properties::RELATIONSHIP_IS_IMPLIED`]; resolves effective redefinitions by identity.
        pub fn is_implied(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::RELATIONSHIP_IS_IMPLIED, ValueKind::Boolean) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_NAME`]; resolves effective redefinitions by identity.
        pub fn owned_member_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNING_MEMBERSHIP`]; resolves effective redefinitions by identity.
        pub fn owning_membership(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNING_MEMBERSHIP, ValueKind::Reference(classes::OWNING_MEMBERSHIP)) }
        /// Read [`properties::ELEMENT_OWNED_ANNOTATION`]; resolves effective redefinitions by identity.
        pub fn owned_annotation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ANNOTATION, ValueKind::Reference(classes::ANNOTATION)) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_member_element(self) -> Result<ElementId, ViewError> { read::required(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_LIBRARY_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn is_library_element(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_LIBRARY_ELEMENT, ValueKind::Boolean) }
        /// Read [`properties::ELEMENT_DECLARED_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNER`]; resolves effective redefinitions by identity.
        pub fn owner(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_OWNER, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_NAME`]; resolves effective redefinitions by identity.
        pub fn name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_OWNED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_IS_IMPLIED_INCLUDED`]; resolves effective redefinitions by identity.
        pub fn is_implied_included(self) -> Result<bool, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_IS_IMPLIED_INCLUDED, ValueKind::Boolean) }
        /// Read [`properties::RELATIONSHIP_OWNED_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owned_related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_OWNED_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_OWNED_RELATIONSHIP`]; resolves effective redefinitions by identity.
        pub fn owned_relationship(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_OWNED_RELATIONSHIP, ValueKind::Reference(classes::RELATIONSHIP)) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn owned_member_element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::ELEMENT_ALIAS_IDS`]; resolves effective redefinitions by identity.
        pub fn alias_ids(self) -> Result<Option<Values<'m, &'m str>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_ALIAS_IDS, ValueKind::String) }
        /// Read [`properties::ELEMENT_DOCUMENTATION`]; resolves effective redefinitions by identity.
        pub fn documentation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_DOCUMENTATION, ValueKind::Reference(classes::DOCUMENTATION)) }
        /// Read [`properties::RELATIONSHIP_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn related_element(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::RELATIONSHIP_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::ELEMENT_DECLARED_NAME`]; resolves effective redefinitions by identity.
        pub fn declared_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_DECLARED_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_TEXTUAL_REPRESENTATION`]; resolves effective redefinitions by identity.
        pub fn textual_representation(self) -> Result<Option<Values<'m, ElementId>>, ViewError> { read::many(self.id(), self.model(), properties::ELEMENT_TEXTUAL_REPRESENTATION, ValueKind::Reference(classes::TEXTUAL_REPRESENTATION)) }
        /// Read [`properties::ELEMENT_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_ELEMENT_ID`]; resolves effective redefinitions by identity.
        pub fn element_id(self) -> Result<&'m str, ViewError> { read::required(self.id(), self.model(), properties::ELEMENT_ELEMENT_ID, ValueKind::String) }
        /// Read [`properties::RELATIONSHIP_OWNING_RELATED_ELEMENT`]; resolves effective redefinitions by identity.
        pub fn owning_related_element(self) -> Result<Option<ElementId>, ViewError> { read::optional(self.id(), self.model(), properties::RELATIONSHIP_OWNING_RELATED_ELEMENT, ValueKind::Reference(classes::ELEMENT)) }
        /// Read [`properties::OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME`]; resolves effective redefinitions by identity.
        pub fn owned_member_short_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME, ValueKind::String) }
        /// Read [`properties::ELEMENT_QUALIFIED_NAME`]; resolves effective redefinitions by identity.
        pub fn qualified_name(self) -> Result<Option<&'m str>, ViewError> { read::optional(self.id(), self.model(), properties::ELEMENT_QUALIFIED_NAME, ValueKind::String) }
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    #[test]
    fn all_named_ids_match_descriptor_inventory() {
        assert_eq!(metamodel::KERML, crate::descriptors().models[0].id);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-CrossSubsetting").unwrap().1, classes::CROSS_SUBSETTING);
        assert_eq!(views::CrossSubsetting::CLASS, classes::CROSS_SUBSETTING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature").unwrap().1, classes::FEATURE);
        assert_eq!(views::Feature::CLASS, classes::FEATURE);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureChaining").unwrap().1, classes::FEATURE_CHAINING);
        assert_eq!(views::FeatureChaining::CLASS, classes::FEATURE_CHAINING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureInverting").unwrap().1, classes::FEATURE_INVERTING);
        assert_eq!(views::FeatureInverting::CLASS, classes::FEATURE_INVERTING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureTyping").unwrap().1, classes::FEATURE_TYPING);
        assert_eq!(views::FeatureTyping::CLASS, classes::FEATURE_TYPING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-Redefinition").unwrap().1, classes::REDEFINITION);
        assert_eq!(views::Redefinition::CLASS, classes::REDEFINITION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-ReferenceSubsetting").unwrap().1, classes::REFERENCE_SUBSETTING);
        assert_eq!(views::ReferenceSubsetting::CLASS, classes::REFERENCE_SUBSETTING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-Subsetting").unwrap().1, classes::SUBSETTING);
        assert_eq!(views::Subsetting::CLASS, classes::SUBSETTING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Features-TypeFeaturing").unwrap().1, classes::TYPE_FEATURING);
        assert_eq!(views::TypeFeaturing::CLASS, classes::TYPE_FEATURING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Conjugation").unwrap().1, classes::CONJUGATION);
        assert_eq!(views::Conjugation::CLASS, classes::CONJUGATION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Differencing").unwrap().1, classes::DIFFERENCING);
        assert_eq!(views::Differencing::CLASS, classes::DIFFERENCING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Disjoining").unwrap().1, classes::DISJOINING);
        assert_eq!(views::Disjoining::CLASS, classes::DISJOINING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-FeatureMembership").unwrap().1, classes::FEATURE_MEMBERSHIP);
        assert_eq!(views::FeatureMembership::CLASS, classes::FEATURE_MEMBERSHIP);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Intersecting").unwrap().1, classes::INTERSECTING);
        assert_eq!(views::Intersecting::CLASS, classes::INTERSECTING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Multiplicity").unwrap().1, classes::MULTIPLICITY);
        assert_eq!(views::Multiplicity::CLASS, classes::MULTIPLICITY);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Specialization").unwrap().1, classes::SPECIALIZATION);
        assert_eq!(views::Specialization::CLASS, classes::SPECIALIZATION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Type").unwrap().1, classes::TYPE);
        assert_eq!(views::Type::CLASS, classes::TYPE);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Core-Types-Unioning").unwrap().1, classes::UNIONING);
        assert_eq!(views::Unioning::CLASS, classes::UNIONING);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Annotations-AnnotatingElement").unwrap().1, classes::ANNOTATING_ELEMENT);
        assert_eq!(views::AnnotatingElement::CLASS, classes::ANNOTATING_ELEMENT);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation").unwrap().1, classes::ANNOTATION);
        assert_eq!(views::Annotation::CLASS, classes::ANNOTATION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Comment").unwrap().1, classes::COMMENT);
        assert_eq!(views::Comment::CLASS, classes::COMMENT);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Documentation").unwrap().1, classes::DOCUMENTATION);
        assert_eq!(views::Documentation::CLASS, classes::DOCUMENTATION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Annotations-TextualRepresentation").unwrap().1, classes::TEXTUAL_REPRESENTATION);
        assert_eq!(views::TextualRepresentation::CLASS, classes::TEXTUAL_REPRESENTATION);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element").unwrap().1, classes::ELEMENT);
        assert_eq!(views::Element::CLASS, classes::ELEMENT);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship").unwrap().1, classes::RELATIONSHIP);
        assert_eq!(views::Relationship::CLASS, classes::RELATIONSHIP);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import").unwrap().1, classes::IMPORT);
        assert_eq!(views::Import::CLASS, classes::IMPORT);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership").unwrap().1, classes::MEMBERSHIP);
        assert_eq!(views::Membership::CLASS, classes::MEMBERSHIP);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace").unwrap().1, classes::NAMESPACE);
        assert_eq!(views::Namespace::CLASS, classes::NAMESPACE);
        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-OwningMembership").unwrap().1, classes::OWNING_MEMBERSHIP);
        assert_eq!(views::OwningMembership::CLASS, classes::OWNING_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_chainingFeature_chainedFeature-chainedFeature").unwrap().1, properties::A_CHAINING_FEATURE_CHAINED_FEATURE_CHAINED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_chainingFeature_chainedFeatureChaining-chainedFeatureChaining").unwrap().1, properties::A_CHAINING_FEATURE_CHAINED_FEATURE_CHAINING_CHAINED_FEATURE_CHAINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_crossFeature_featureCrossing-featureCrossing").unwrap().1, properties::A_CROSS_FEATURE_FEATURE_CROSSING_FEATURE_CROSSING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_crossedFeature_crossSupersetting-crossSupersetting").unwrap().1, properties::A_CROSSED_FEATURE_CROSS_SUPERSETTING_CROSS_SUPERSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_featureOfType_typeFeaturing-typeFeaturing").unwrap().1, properties::A_FEATURE_OF_TYPE_TYPE_FEATURING_TYPE_FEATURING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_featureTarget_baseFeature-baseFeature").unwrap().1, properties::A_FEATURE_TARGET_BASE_FEATURE_BASE_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_featuringType_featureOfType-featureOfType").unwrap().1, properties::A_FEATURING_TYPE_FEATURE_OF_TYPE_FEATURE_OF_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_featuringType_typeFeaturingOfType-typeFeaturingOfType").unwrap().1, properties::A_FEATURING_TYPE_TYPE_FEATURING_OF_TYPE_TYPE_FEATURING_OF_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_invertingFeatureInverting_featureInverted-invertingFeatureInverting").unwrap().1, properties::A_INVERTING_FEATURE_INVERTING_FEATURE_INVERTED_INVERTING_FEATURE_INVERTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_invertingFeature_invertedFeatureInverting-invertedFeatureInverting").unwrap().1, properties::A_INVERTING_FEATURE_INVERTED_FEATURE_INVERTING_INVERTED_FEATURE_INVERTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_multiplicity_typeWithMultiplicity-typeWithMultiplicity").unwrap().1, properties::A_MULTIPLICITY_TYPE_WITH_MULTIPLICITY_TYPE_WITH_MULTIPLICITY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_ownedRedefinition_owningFeature-owningFeature").unwrap().1, properties::A_OWNED_REDEFINITION_OWNING_FEATURE_OWNING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_redefinedFeature_redefining-redefining").unwrap().1, properties::A_REDEFINED_FEATURE_REDEFINING_REDEFINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_redefiningFeature_redefinition-redefinition").unwrap().1, properties::A_REDEFINING_FEATURE_REDEFINITION_REDEFINITION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_referencedFeature_referencing-referencing").unwrap().1, properties::A_REFERENCED_FEATURE_REFERENCING_REFERENCING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_subsettedFeature_supersetting-supersetting").unwrap().1, properties::A_SUBSETTED_FEATURE_SUPERSETTING_SUPERSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_subsettingFeature_subsetting-subsetting").unwrap().1, properties::A_SUBSETTING_FEATURE_SUBSETTING_SUBSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_type_typingByType-typingByType").unwrap().1, properties::A_TYPE_TYPING_BY_TYPE_TYPING_BY_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_typedFeature_type-typedFeature").unwrap().1, properties::A_TYPED_FEATURE_TYPE_TYPED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-A_typing_typedFeature-typing").unwrap().1, properties::A_TYPING_TYPED_FEATURE_TYPING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-CrossSubsetting-crossedFeature").unwrap().1, properties::CROSS_SUBSETTING_CROSSED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-CrossSubsetting-crossingFeature").unwrap().1, properties::CROSS_SUBSETTING_CROSSING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-chainingFeature").unwrap().1, properties::FEATURE_CHAINING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-crossFeature").unwrap().1, properties::FEATURE_CROSS_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-direction").unwrap().1, properties::FEATURE_DIRECTION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-endOwningType").unwrap().1, properties::FEATURE_END_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-featureTarget").unwrap().1, properties::FEATURE_FEATURE_TARGET);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-featuringType").unwrap().1, properties::FEATURE_FEATURING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isComposite").unwrap().1, properties::FEATURE_IS_COMPOSITE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isConstant").unwrap().1, properties::FEATURE_IS_CONSTANT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isDerived").unwrap().1, properties::FEATURE_IS_DERIVED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isEnd").unwrap().1, properties::FEATURE_IS_END);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isOrdered").unwrap().1, properties::FEATURE_IS_ORDERED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isPortion").unwrap().1, properties::FEATURE_IS_PORTION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isUnique").unwrap().1, properties::FEATURE_IS_UNIQUE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-isVariable").unwrap().1, properties::FEATURE_IS_VARIABLE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedCrossSubsetting").unwrap().1, properties::FEATURE_OWNED_CROSS_SUBSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedFeatureChaining").unwrap().1, properties::FEATURE_OWNED_FEATURE_CHAINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedFeatureInverting").unwrap().1, properties::FEATURE_OWNED_FEATURE_INVERTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedRedefinition").unwrap().1, properties::FEATURE_OWNED_REDEFINITION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedReferenceSubsetting").unwrap().1, properties::FEATURE_OWNED_REFERENCE_SUBSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedSubsetting").unwrap().1, properties::FEATURE_OWNED_SUBSETTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedTypeFeaturing").unwrap().1, properties::FEATURE_OWNED_TYPE_FEATURING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-ownedTyping").unwrap().1, properties::FEATURE_OWNED_TYPING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-owningFeatureMembership").unwrap().1, properties::FEATURE_OWNING_FEATURE_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-owningType").unwrap().1, properties::FEATURE_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Feature-type").unwrap().1, properties::FEATURE_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureChaining-chainingFeature").unwrap().1, properties::FEATURE_CHAINING_CHAINING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureChaining-featureChained").unwrap().1, properties::FEATURE_CHAINING_FEATURE_CHAINED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureInverting-featureInverted").unwrap().1, properties::FEATURE_INVERTING_FEATURE_INVERTED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureInverting-invertingFeature").unwrap().1, properties::FEATURE_INVERTING_INVERTING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureInverting-owningFeature").unwrap().1, properties::FEATURE_INVERTING_OWNING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureTyping-owningFeature").unwrap().1, properties::FEATURE_TYPING_OWNING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureTyping-type").unwrap().1, properties::FEATURE_TYPING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-FeatureTyping-typedFeature").unwrap().1, properties::FEATURE_TYPING_TYPED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Redefinition-redefinedFeature").unwrap().1, properties::REDEFINITION_REDEFINED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Redefinition-redefiningFeature").unwrap().1, properties::REDEFINITION_REDEFINING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-ReferenceSubsetting-referencedFeature").unwrap().1, properties::REFERENCE_SUBSETTING_REFERENCED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-ReferenceSubsetting-referencingFeature").unwrap().1, properties::REFERENCE_SUBSETTING_REFERENCING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Subsetting-owningFeature").unwrap().1, properties::SUBSETTING_OWNING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Subsetting-subsettedFeature").unwrap().1, properties::SUBSETTING_SUBSETTED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-Subsetting-subsettingFeature").unwrap().1, properties::SUBSETTING_SUBSETTING_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-TypeFeaturing-featureOfType").unwrap().1, properties::TYPE_FEATURING_FEATURE_OF_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-TypeFeaturing-featuringType").unwrap().1, properties::TYPE_FEATURING_FEATURING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Features-TypeFeaturing-owningFeatureOfType").unwrap().1, properties::TYPE_FEATURING_OWNING_FEATURE_OF_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_conjugatedType_conjugator-conjugator").unwrap().1, properties::A_CONJUGATED_TYPE_CONJUGATOR_CONJUGATOR);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_differencingType_differencedDifferencing-differencedDifferencing").unwrap().1, properties::A_DIFFERENCING_TYPE_DIFFERENCED_DIFFERENCING_DIFFERENCED_DIFFERENCING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_differencingType_differencedType-differencedType").unwrap().1, properties::A_DIFFERENCING_TYPE_DIFFERENCED_TYPE_DIFFERENCED_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_directedFeature_typeWithDirectedFeature-typeWithDirectedFeature").unwrap().1, properties::A_DIRECTED_FEATURE_TYPE_WITH_DIRECTED_FEATURE_TYPE_WITH_DIRECTED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_disjoiningTypeDisjoining_typeDisjoined-disjoiningTypeDisjoining").unwrap().1, properties::A_DISJOINING_TYPE_DISJOINING_TYPE_DISJOINED_DISJOINING_TYPE_DISJOINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_disjoiningType_disjoinedTypeDisjoining-disjoinedTypeDisjoining").unwrap().1, properties::A_DISJOINING_TYPE_DISJOINED_TYPE_DISJOINING_DISJOINED_TYPE_DISJOINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_endFeature_typeWithEndFeature-typeWithEndFeature").unwrap().1, properties::A_END_FEATURE_TYPE_WITH_END_FEATURE_TYPE_WITH_END_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_featureMembership_type-type").unwrap().1, properties::A_FEATURE_MEMBERSHIP_TYPE_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_general_generalization-generalization").unwrap().1, properties::A_GENERAL_GENERALIZATION_GENERALIZATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_inheritedFeature_inheritingType-inheritingType").unwrap().1, properties::A_INHERITED_FEATURE_INHERITING_TYPE_INHERITING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_inheritedMembership_inheritingType-inheritingType").unwrap().1, properties::A_INHERITED_MEMBERSHIP_INHERITING_TYPE_INHERITING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_input_typeWithInput-typeWithInput").unwrap().1, properties::A_INPUT_TYPE_WITH_INPUT_TYPE_WITH_INPUT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_intersectingType_intersectedIntersecting-intersectedIntersecting").unwrap().1, properties::A_INTERSECTING_TYPE_INTERSECTED_INTERSECTING_INTERSECTED_INTERSECTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_intersectingType_intersectedType-intersectedType").unwrap().1, properties::A_INTERSECTING_TYPE_INTERSECTED_TYPE_INTERSECTED_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_originalType_conjugation-conjugation").unwrap().1, properties::A_ORIGINAL_TYPE_CONJUGATION_CONJUGATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_output_typeWithOutput-typeWithOutput").unwrap().1, properties::A_OUTPUT_TYPE_WITH_OUTPUT_TYPE_WITH_OUTPUT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_specific_specialization-specialization").unwrap().1, properties::A_SPECIFIC_SPECIALIZATION_SPECIALIZATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_typeWithFeature_feature-typeWithFeature").unwrap().1, properties::A_TYPE_WITH_FEATURE_FEATURE_TYPE_WITH_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_unioningType_unionedType-unionedType").unwrap().1, properties::A_UNIONING_TYPE_UNIONED_TYPE_UNIONED_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-A_unioningType_unionedUnioning-unionedUnioning").unwrap().1, properties::A_UNIONING_TYPE_UNIONED_UNIONING_UNIONED_UNIONING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Conjugation-conjugatedType").unwrap().1, properties::CONJUGATION_CONJUGATED_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Conjugation-originalType").unwrap().1, properties::CONJUGATION_ORIGINAL_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Conjugation-owningType").unwrap().1, properties::CONJUGATION_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Differencing-differencingType").unwrap().1, properties::DIFFERENCING_DIFFERENCING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Differencing-typeDifferenced").unwrap().1, properties::DIFFERENCING_TYPE_DIFFERENCED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Disjoining-disjoiningType").unwrap().1, properties::DISJOINING_DISJOINING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Disjoining-owningType").unwrap().1, properties::DISJOINING_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Disjoining-typeDisjoined").unwrap().1, properties::DISJOINING_TYPE_DISJOINED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-FeatureMembership-ownedMemberFeature").unwrap().1, properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-FeatureMembership-owningType").unwrap().1, properties::FEATURE_MEMBERSHIP_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Intersecting-intersectingType").unwrap().1, properties::INTERSECTING_INTERSECTING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Intersecting-typeIntersected").unwrap().1, properties::INTERSECTING_TYPE_INTERSECTED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Specialization-general").unwrap().1, properties::SPECIALIZATION_GENERAL);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Specialization-owningType").unwrap().1, properties::SPECIALIZATION_OWNING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Specialization-specific").unwrap().1, properties::SPECIALIZATION_SPECIFIC);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-differencingType").unwrap().1, properties::TYPE_DIFFERENCING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-directedFeature").unwrap().1, properties::TYPE_DIRECTED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-endFeature").unwrap().1, properties::TYPE_END_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-feature").unwrap().1, properties::TYPE_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-featureMembership").unwrap().1, properties::TYPE_FEATURE_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-inheritedFeature").unwrap().1, properties::TYPE_INHERITED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-inheritedMembership").unwrap().1, properties::TYPE_INHERITED_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-input").unwrap().1, properties::TYPE_INPUT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-intersectingType").unwrap().1, properties::TYPE_INTERSECTING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-isAbstract").unwrap().1, properties::TYPE_IS_ABSTRACT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-isConjugated").unwrap().1, properties::TYPE_IS_CONJUGATED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-isSufficient").unwrap().1, properties::TYPE_IS_SUFFICIENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-multiplicity").unwrap().1, properties::TYPE_MULTIPLICITY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-output").unwrap().1, properties::TYPE_OUTPUT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedConjugator").unwrap().1, properties::TYPE_OWNED_CONJUGATOR);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedDifferencing").unwrap().1, properties::TYPE_OWNED_DIFFERENCING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedDisjoining").unwrap().1, properties::TYPE_OWNED_DISJOINING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedEndFeature").unwrap().1, properties::TYPE_OWNED_END_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedFeature").unwrap().1, properties::TYPE_OWNED_FEATURE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedFeatureMembership").unwrap().1, properties::TYPE_OWNED_FEATURE_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedIntersecting").unwrap().1, properties::TYPE_OWNED_INTERSECTING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedSpecialization").unwrap().1, properties::TYPE_OWNED_SPECIALIZATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-ownedUnioning").unwrap().1, properties::TYPE_OWNED_UNIONING);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Type-unioningType").unwrap().1, properties::TYPE_UNIONING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Unioning-typeUnioned").unwrap().1, properties::UNIONING_TYPE_UNIONED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Core-Types-Unioning-unioningType").unwrap().1, properties::UNIONING_UNIONING_TYPE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-A_annotatedElement_annotatingElement-annotatingElement").unwrap().1, properties::A_ANNOTATED_ELEMENT_ANNOTATING_ELEMENT_ANNOTATING_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-A_annotatedElement_annotation-annotation").unwrap().1, properties::A_ANNOTATED_ELEMENT_ANNOTATION_ANNOTATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-AnnotatingElement-annotatedElement").unwrap().1, properties::ANNOTATING_ELEMENT_ANNOTATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-AnnotatingElement-annotation").unwrap().1, properties::ANNOTATING_ELEMENT_ANNOTATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-AnnotatingElement-ownedAnnotatingRelationship").unwrap().1, properties::ANNOTATING_ELEMENT_OWNED_ANNOTATING_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-AnnotatingElement-owningAnnotatingRelationship").unwrap().1, properties::ANNOTATING_ELEMENT_OWNING_ANNOTATING_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation-annotatedElement").unwrap().1, properties::ANNOTATION_ANNOTATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation-annotatingElement").unwrap().1, properties::ANNOTATION_ANNOTATING_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation-ownedAnnotatingElement").unwrap().1, properties::ANNOTATION_OWNED_ANNOTATING_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation-owningAnnotatedElement").unwrap().1, properties::ANNOTATION_OWNING_ANNOTATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Annotation-owningAnnotatingElement").unwrap().1, properties::ANNOTATION_OWNING_ANNOTATING_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Comment-body").unwrap().1, properties::COMMENT_BODY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Comment-locale").unwrap().1, properties::COMMENT_LOCALE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-Documentation-documentedElement").unwrap().1, properties::DOCUMENTATION_DOCUMENTED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-TextualRepresentation-body").unwrap().1, properties::TEXTUAL_REPRESENTATION_BODY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-TextualRepresentation-language").unwrap().1, properties::TEXTUAL_REPRESENTATION_LANGUAGE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Annotations-TextualRepresentation-representedElement").unwrap().1, properties::TEXTUAL_REPRESENTATION_REPRESENTED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-A_relatedElement_relationship-relationship").unwrap().1, properties::A_RELATED_ELEMENT_RELATIONSHIP_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-A_source_sourceRelationship-sourceRelationship").unwrap().1, properties::A_SOURCE_SOURCE_RELATIONSHIP_SOURCE_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-A_target_targetRelationship-targetRelationship").unwrap().1, properties::A_TARGET_TARGET_RELATIONSHIP_TARGET_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-aliasIds").unwrap().1, properties::ELEMENT_ALIAS_IDS);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-declaredName").unwrap().1, properties::ELEMENT_DECLARED_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-declaredShortName").unwrap().1, properties::ELEMENT_DECLARED_SHORT_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-documentation").unwrap().1, properties::ELEMENT_DOCUMENTATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-elementId").unwrap().1, properties::ELEMENT_ELEMENT_ID);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-isImpliedIncluded").unwrap().1, properties::ELEMENT_IS_IMPLIED_INCLUDED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-isLibraryElement").unwrap().1, properties::ELEMENT_IS_LIBRARY_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-name").unwrap().1, properties::ELEMENT_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-ownedAnnotation").unwrap().1, properties::ELEMENT_OWNED_ANNOTATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-ownedElement").unwrap().1, properties::ELEMENT_OWNED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-ownedRelationship").unwrap().1, properties::ELEMENT_OWNED_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-owner").unwrap().1, properties::ELEMENT_OWNER);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-owningMembership").unwrap().1, properties::ELEMENT_OWNING_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-owningNamespace").unwrap().1, properties::ELEMENT_OWNING_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-owningRelationship").unwrap().1, properties::ELEMENT_OWNING_RELATIONSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-qualifiedName").unwrap().1, properties::ELEMENT_QUALIFIED_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-shortName").unwrap().1, properties::ELEMENT_SHORT_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Element-textualRepresentation").unwrap().1, properties::ELEMENT_TEXTUAL_REPRESENTATION);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-isImplied").unwrap().1, properties::RELATIONSHIP_IS_IMPLIED);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-ownedRelatedElement").unwrap().1, properties::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-owningRelatedElement").unwrap().1, properties::RELATIONSHIP_OWNING_RELATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-relatedElement").unwrap().1, properties::RELATIONSHIP_RELATED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-source").unwrap().1, properties::RELATIONSHIP_SOURCE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Elements-Relationship-target").unwrap().1, properties::RELATIONSHIP_TARGET);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-A_importedElement_membershipImport-membershipImport").unwrap().1, properties::A_IMPORTED_ELEMENT_MEMBERSHIP_IMPORT_MEMBERSHIP_IMPORT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-A_importedMembership_importingNamespace-importingNamespace").unwrap().1, properties::A_IMPORTED_MEMBERSHIP_IMPORTING_NAMESPACE_IMPORTING_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-A_memberElement_membership-membership").unwrap().1, properties::A_MEMBER_ELEMENT_MEMBERSHIP_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-A_member_namespace-namespace").unwrap().1, properties::A_MEMBER_NAMESPACE_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-A_membership_membershipNamespace-membershipNamespace").unwrap().1, properties::A_MEMBERSHIP_MEMBERSHIP_NAMESPACE_MEMBERSHIP_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import-importOwningNamespace").unwrap().1, properties::IMPORT_IMPORT_OWNING_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import-importedElement").unwrap().1, properties::IMPORT_IMPORTED_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import-isImportAll").unwrap().1, properties::IMPORT_IS_IMPORT_ALL);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import-isRecursive").unwrap().1, properties::IMPORT_IS_RECURSIVE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Import-visibility").unwrap().1, properties::IMPORT_VISIBILITY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-memberElement").unwrap().1, properties::MEMBERSHIP_MEMBER_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-memberElementId").unwrap().1, properties::MEMBERSHIP_MEMBER_ELEMENT_ID);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-memberName").unwrap().1, properties::MEMBERSHIP_MEMBER_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-memberShortName").unwrap().1, properties::MEMBERSHIP_MEMBER_SHORT_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-membershipOwningNamespace").unwrap().1, properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Membership-visibility").unwrap().1, properties::MEMBERSHIP_VISIBILITY);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-importedMembership").unwrap().1, properties::NAMESPACE_IMPORTED_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-member").unwrap().1, properties::NAMESPACE_MEMBER);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-membership").unwrap().1, properties::NAMESPACE_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-ownedImport").unwrap().1, properties::NAMESPACE_OWNED_IMPORT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-ownedMember").unwrap().1, properties::NAMESPACE_OWNED_MEMBER);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-Namespace-ownedMembership").unwrap().1, properties::NAMESPACE_OWNED_MEMBERSHIP);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-OwningMembership-ownedMemberElement").unwrap().1, properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-OwningMembership-ownedMemberElementId").unwrap().1, properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT_ID);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-OwningMembership-ownedMemberName").unwrap().1, properties::OWNING_MEMBERSHIP_OWNED_MEMBER_NAME);
        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == "Root-Namespaces-OwningMembership-ownedMemberShortName").unwrap().1, properties::OWNING_MEMBERSHIP_OWNED_MEMBER_SHORT_NAME);
    }
}
