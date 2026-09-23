use super::*;

#[test]
fn interfaces_inline_function_body_preserves_frontend_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|source| source.path() == "Systems Library/Interfaces.sysml")
        .unwrap();
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        source.document(),
        source.revision(),
        source.source(),
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete());
    let draft = construction::construct_on(
        &[SourceInput {
            syntax: &syntax,
            library: Some(source),
            sysml: true,
        }],
        &Default::default(),
        agq_kerml::BaselineProfile::OPERATIONAL_V9,
        Snapshot::new(Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        )),
        None,
    )
    .unwrap();
    let model = draft.candidate().model();
    let id = ElementId::from_u128;
    let references = |element, property| {
        model
            .navigation_slot(element, property)
            .into_iter()
            .flat_map(|slot| slot.value().values())
            .filter_map(|value| {
                if let Value::Reference(id) = value {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    let child = |owner, membership, expected| {
        let matches = references(owner, p::ELEMENT_OWNED_RELATIONSHIP)
            .into_iter()
            .filter(|&relationship| model.element(relationship).unwrap().metaclass() == membership)
            .filter(|&relationship| {
                references(relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT) == [expected]
            })
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "{owner}/{expected}");
        matches[0]
    };
    let position = id(0xa0c20813d2485d4a8468ba344c8f229d);
    let invocation = id(0x4dd5decc51395b11a577188e3d792728);
    assert_eq!(
        model.element(position).unwrap().metaclass(),
        agq_sysml::classes::ATTRIBUTE_USAGE
    );
    assert_eq!(
        model.element(invocation).unwrap().metaclass(),
        c::INVOCATION_EXPRESSION
    );
    child(position, c::FEATURE_VALUE, invocation);
    let argument = id(0xe61113b4174f583d957c33931b39c4b8);
    let body_reference = id(0x01339ce6389f564a96b23f68188cb289);
    let body = id(0xfa01f3cef1075cffb8eb1b041f865002);
    let input = id(0xbd4b93a877f85a309d692e260023f949);
    let equality = id(0x5cc6bbda4c3558e1bcb480a011ba82bc);
    child(invocation, c::PARAMETER_MEMBERSHIP, argument);
    child(argument, c::FEATURE_VALUE, body_reference);
    child(body_reference, c::FEATURE_MEMBERSHIP, body);
    child(body, c::FEATURE_MEMBERSHIP, input);
    child(body, c::RESULT_EXPRESSION_MEMBERSHIP, equality);
    for (element, class) in [
        (argument, c::FEATURE),
        (body_reference, c::FEATURE_REFERENCE_EXPRESSION),
        (body, c::EXPRESSION),
        (input, c::FEATURE),
        (equality, c::OPERATOR_EXPRESSION),
    ] {
        assert_eq!(model.element(element).unwrap().metaclass(), class);
    }
    assert_eq!(
        syntax.text(
            draft
                .source_map()
                .get(&FactKey::Element(body))
                .unwrap()
                .range
        ),
        Some("{in i; seq#(i) == value}")
    );
    assert_eq!(
        syntax.text(
            draft
                .source_map()
                .get(&FactKey::Element(input))
                .unwrap()
                .range
        ),
        Some("in i;")
    );
    let index = id(0x6f0aaf9870d8529a89432dad7d972817);
    assert_eq!(
        model.element(index).unwrap().metaclass(),
        c::INDEX_EXPRESSION
    );
    let index_argument = id(0x2bf8cc7df4325dd2bc4766d743b942ee);
    child(equality, c::PARAMETER_MEMBERSHIP, index_argument);
    child(index_argument, c::FEATURE_VALUE, index);
    for (owner, result, implied) in [
        (body_reference, id(0x3368a31d9c8957f080107f8974f5c3d5), true),
        (body, id(0x4a4ad630e7995409ba497d684f99543e), true),
        (index, id(0x672001a8594b5b80ac7c590764d0540b), true),
        (equality, id(0x76e1403900ec58ae86a2c6ecc9e83a00), false),
        (invocation, id(0x40bc5d56e1995e20aa2f59b963265042), false),
    ] {
        let membership = child(owner, c::RETURN_PARAMETER_MEMBERSHIP, result);
        assert_eq!(
            model.element(result).unwrap().metaclass(),
            if implied {
                c::FEATURE
            } else {
                agq_sysml::classes::REFERENCE_USAGE
            }
        );
        assert_eq!(
            model
                .navigation_slot(membership, p::RELATIONSHIP_IS_IMPLIED)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Boolean(implied))
        );
    }
    for (expression, member, name) in [
        (
            id(0xfc7c7d2462fd5769a22cbd9fcde9823c),
            id(0x9de0675b12a95a618db1dba6c11d3a4a),
            "seq",
        ),
        (
            id(0x0d8820e7c28e50968a79ee8efdc81a53),
            id(0x6a45d227d0635d0f86935e2aa80dc0d8),
            "i",
        ),
        (
            id(0xa0dfce7c74df525188a0d3b36ee2476c),
            id(0xbd831c6b33895c5a9d92fb017f98b95a),
            "value",
        ),
    ] {
        assert_eq!(
            model.element(expression).unwrap().metaclass(),
            c::FEATURE_REFERENCE_EXPRESSION
        );
        assert!(references(expression, p::ELEMENT_OWNED_RELATIONSHIP).contains(&member));
        let reference = draft
            .references()
            .iter()
            .find(|reference| reference.relationship == member)
            .unwrap();
        assert_eq!(reference.name.segments, [name]);
    }
    // The full callee has no own return: it inherits Calculation/Evaluation's
    // result. Its final body invocation owns the explicit SysML empty result.
    let named = |name: &str| {
        model
            .elements()
            .find(|record| {
                record.slot(p::ELEMENT_DECLARED_NAME).is_some_and(|slot| {
                    slot.value()
                        .values()
                        .any(|value| value == &Value::String(name.into()))
                })
            })
            .unwrap()
            .id()
    };
    let owned = |owner, class| {
        references(owner, p::ELEMENT_OWNED_RELATIONSHIP)
            .into_iter()
            .filter(|relationship| model.element(*relationship).unwrap().metaclass() == class)
            .flat_map(|relationship| {
                references(relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
            })
            .collect::<Vec<_>>()
    };
    let calculation = named("excludingOnce");
    let seq = named("seq");
    let value = named("value");
    assert_eq!(
        model.element(calculation).unwrap().metaclass(),
        agq_sysml::classes::CALCULATION_DEFINITION
    );
    assert!(owned(calculation, c::RETURN_PARAMETER_MEMBERSHIP).is_empty());
    for (feature, bounds) in [
        (seq, vec![c::LITERAL_INTEGER, c::LITERAL_INFINITY]),
        (value, vec![c::LITERAL_INTEGER]),
    ] {
        child(calculation, c::FEATURE_MEMBERSHIP, feature);
        assert_eq!(
            model.element(feature).unwrap().metaclass(),
            agq_sysml::classes::REFERENCE_USAGE
        );
        let range = owned(feature, c::OWNING_MEMBERSHIP)
            .into_iter()
            .find(|element| model.element(*element).unwrap().metaclass() == c::MULTIPLICITY_RANGE)
            .unwrap();
        let literals = owned(range, c::OWNING_MEMBERSHIP);
        assert_eq!(
            literals
                .iter()
                .map(|element| model.element(*element).unwrap().metaclass())
                .collect::<Vec<_>>(),
            bounds
        );
        for literal in literals {
            let results = owned(literal, c::RETURN_PARAMETER_MEMBERSHIP);
            assert_eq!(results.len(), 1);
            assert_eq!(model.element(results[0]).unwrap().metaclass(), c::FEATURE);
        }
    }
    let subset = references(value, p::ELEMENT_OWNED_RELATIONSHIP)
        .into_iter()
        .find(|element| model.element(*element).unwrap().metaclass() == c::SUBSETTING)
        .unwrap();
    assert_eq!(
        draft
            .references()
            .iter()
            .find(|reference| reference.relationship == subset)
            .unwrap()
            .name
            .segments,
        ["seq"]
    );
    let range = id(0xdefd4cf3013a55228498bf56a584ab11);
    let first = id(0x8dad8aad0fef5d9b9727ae371dbc1107);
    child(invocation, c::PARAMETER_MEMBERSHIP, first);
    child(first, c::FEATURE_VALUE, range);
    assert_eq!(
        model.element(range).unwrap().metaclass(),
        c::OPERATOR_EXPRESSION
    );
    let size = id(0x4d8fd9ed78e153db809ae4afc4f30361);
    let upper = id(0x52ecf4096e5e546c8b7813f901d2dc77);
    child(range, c::PARAMETER_MEMBERSHIP, upper);
    child(upper, c::FEATURE_VALUE, size);
    assert_eq!(
        model.element(size).unwrap().metaclass(),
        c::INVOCATION_EXPRESSION
    );
    let final_call = id(0xbe40b5edd16f54978960a1116667f53f);
    child(calculation, c::RESULT_EXPRESSION_MEMBERSHIP, final_call);
    assert_eq!(
        model.element(final_call).unwrap().metaclass(),
        c::INVOCATION_EXPRESSION
    );
    assert_eq!(owned(final_call, c::PARAMETER_MEMBERSHIP).len(), 2);
    for expression in [range, size, final_call] {
        let results = owned(expression, c::RETURN_PARAMETER_MEMBERSHIP);
        assert_eq!(results.len(), 1);
        assert_eq!(
            model.element(results[0]).unwrap().metaclass(),
            agq_sysml::classes::REFERENCE_USAGE
        );
    }
}
