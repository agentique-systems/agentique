"""Independent local facts for remaining producer/positional work, not waivers."""
import collections
import json
import sys
from corpus import *

p=Pinned();m=reference()
audit_path=OUT/'full-v7-publication.json'
audit=json.loads(audit_path.read_text(encoding='utf8'))
contexts=[]
for diagnostic in audit['producer_diagnostics']:
    if diagnostic['code']!='KQ_REFERENCE_CONTEXT':continue
    expression=diagnostic['subject'];bound_membership=p.parents[expression]
    multiplicity=p.parents[bound_membership]
    assert p.is_kind(bound_membership,'OwningMembership') and p.is_kind(multiplicity,'MultiplicityRange')
    multiplicity_membership=p.parents[multiplicity];feature=p.parents[multiplicity_membership]
    assert not p.is_kind(multiplicity_membership,'FeatureMembership') and p.is_kind(feature,'Feature')
    membership=p.parents.get(feature)
    feature_owner=p.parents.get(membership)
    ordinary=p.is_kind(membership,'FeatureMembership')
    cross=not ordinary and p.is_kind(feature_owner,'Feature') and p.select_cross(feature_owner)==feature
    assert ordinary or cross
    contexts.append(dict(expression=p.brief(expression),bound_membership=p.brief(bound_membership),
        multiplicity=p.brief(multiplicity),multiplicity_membership=p.brief(multiplicity_membership),
        feature=p.brief(feature),feature_membership=p.brief(membership),feature_owner=p.brief(feature_owner),
        feature_is_variable=p.scalar(feature,'isVariable'),ordinary_owned_feature=ordinary,owned_cross_feature=cross,
        root_cause=('KERML11-4: multiplicity domain is disputed; no containing-owner domain is silently selected.' if ordinary else
                    'Cross-feature multiplicity/bound domain also needs the pending KERML11-3 analysis; no alternative domain is silently selected.')))
assert len(contexts)==15

def relevant_parameters(x):
    return [v for r,v in members(m,x) if kind(m,r,'FeatureMembership') and v.get('direction') and not kind(m,r,'ReturnParameterMembership')]

triggers=[]
for source_id in ['4a74747a-c385-5a89-89c8-be2fadaa9ecb','518b9181-2fd8-5bfa-953c-3292058bc119']:
    # Match canonical owner plus the source's local parameter names; reference
    # generated IDs are recorded rather than treated as semantic identities.
    owner=p.owner(source_id);owner_ref=m.find(p.path(owner))
    steps=[v for r,v in members(m,owner_ref) if kind(m,v,'Step') and not m.name(v)
           and [m.name(q) for q in relevant_parameters(v)]==['observer','signal']]
    assert len(steps)==1,(p.path(owner),len(steps))
    step=steps[0]
    generals=related(m,step,'Subsetting','subsettedFeature')
    general_rows=[]
    for general in generals:
        chain=related(m,general,'FeatureChaining','chainingFeature')
        general_rows.append(dict(type=brief(m,general),owned_parameters=[brief(m,v) for v in relevant_parameters(general)],
            chain=[brief(m,v) for v in chain],terminal_owned_parameters=[brief(m,v) for v in relevant_parameters(chain[-1])] if chain else []))
    local=[dict(parameter=brief(m,v),redefined=[brief(m,t) for t in related(m,v,'Redefinition','redefinedFeature')]) for v in relevant_parameters(step)]
    assert all(r['redefined'] for r in local)
    triggers.append(dict(pinned=p.brief(source_id),reference=brief(m,step),direct_generals=general_rows,parameters=local,
        conclusion='Current reference parameters redefine the chain-terminal parameters. Published checkFeatureParameterRedefinition selects direct-general owned parameters; the pilot selects all parameters. The ordinary canonical parameter/chain closure remains unfinished. This trace alone is not a new source/library erratum or an authority waiver.'))

formal=[r for r in inventory['members'] if r['name'] in ['checkFeatureParameterRedefinition','checkFeatureResultRedefinition','checkFeatureEndRedefinition','checkMultiplicityRangeBoundTypeFeaturing','checkMultiplicityTypeFeaturing','validateFeatureMultiplicityDomain']]
report=dict(format='agentique-v10-remaining-structural-investigations/1',
    source_audit=dict(path=audit_path.relative_to(ROOT).as_posix(),sha256=digest(audit_path)),
    source_archive_sha256=digest(p.archive),formal=formal,
    reference_binding_contexts=contexts,
    context_groups=dict(collections.Counter('ordinary_multiplicity' if r['ordinary_owned_feature'] else 'cross_multiplicity' for r in contexts)),
    triggers=triggers,ordinary_positional_closure_complete=False,additional_corrections_applied=False)
encoded=json.dumps(report,indent=2,ensure_ascii=False)+'\n';path=OUT/'remaining-structural-investigations.json'
if '--check' in sys.argv:assert path.read_text(encoding='utf8')==encoded
else:path.write_text(encoded,encoding='utf8',newline='\n')
print('Independently traced all 15 reference-binding context findings to multiplicity bounds and both Triggers parameter-redefinition paths; no waiver or correction applied.')
