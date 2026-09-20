include!("namespace_fixture.rs");

#[allow(dead_code)]
fn member(f: &mut Fixture, owner: u128, member: u128, rel: u128, class: MetaclassId) {
    f.create(rel, class);
    f.own(owner, rel);
    f.changes.set(
        id(rel),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(member))]),
        origin(),
    );
}

#[allow(dead_code)]
fn relation(
    f: &mut Fixture,
    specific: u128,
    general: u128,
    rel: u128,
    class: MetaclassId,
    property: PropertyId,
) {
    f.create(rel, class);
    f.own(specific, rel);
    f.value(rel, property, Value::Reference(id(general)));
}

#[allow(dead_code)]
fn subset(f: &mut Fixture, specific: u128, general: u128, rel: u128) {
    relation(
        f,
        specific,
        general,
        rel,
        c::SUBSETTING,
        p::SUBSETTING_SUBSETTED_FEATURE,
    );
    f.value(
        rel,
        p::SUBSETTING_SUBSETTING_FEATURE,
        Value::Reference(id(specific)),
    );
}

#[allow(dead_code)]
fn type_featuring(f: &mut Fixture, feature: u128, ty: u128, rel: u128) {
    relation(
        f,
        feature,
        ty,
        rel,
        c::TYPE_FEATURING,
        p::TYPE_FEATURING_FEATURING_TYPE,
    );
    f.value(
        rel,
        p::TYPE_FEATURING_FEATURE_OF_TYPE,
        Value::Reference(id(feature)),
    );
}

#[allow(dead_code)]
fn redefine(f: &mut Fixture, specific: u128, general: u128, rel: u128) {
    relation(
        f,
        specific,
        general,
        rel,
        c::REDEFINITION,
        p::REDEFINITION_REDEFINED_FEATURE,
    );
    f.value(
        rel,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(specific)),
    );
}

#[allow(dead_code)]
fn binding(f: &mut Fixture, owner: u128, n: u128, membership: MetaclassId, a: u128, b: u128) {
    f.create(n, c::BINDING_CONNECTOR);
    member(f, owner, n, n + 1, membership);
    for (offset, endpoint) in [(2, a), (5, b)] {
        let end = n + offset;
        f.create(end, c::FEATURE);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(f, n, end, end + 1, c::END_FEATURE_MEMBERSHIP);
        relation(
            f,
            end,
            endpoint,
            end + 2,
            c::REFERENCE_SUBSETTING,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
}

