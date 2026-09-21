"""Complete independent source populations for remaining structural audits.

Ordered projections use the canonical exported ownedRelationship sequence.
This source inventory never claims that missing implied relationships exist.
"""
import collections
import json
import sys
from corpus import *

p = Pinned()
assertions = {r['relationship']: r for r in p.export['active_references']}
values, chains, positions = [], [], []

def named_assertion(relationship):
    assertion = assertions.get(relationship)
    return dict(name=assertion['name'], absolute=assertion['absolute'], property=assertion['property']) if assertion else None

for element in p.records:
    if p.is_kind(element, 'FeatureValue'):
        owner = p.parents[element]
        expressions = p.refs(element, 'ownedRelatedElement')
        assert p.is_kind(owner, 'Feature') and len(expressions) == 1
        expression = expressions[0]
        assert p.is_kind(expression, 'Expression')
        values.append(dict(membership=element, feature_with_value=p.brief(owner), value=p.brief(expression),
            is_default=p.scalar(element, 'isDefault'), is_initial=p.scalar(element, 'isInitial'),
            membership_owned_index=p.refs(owner, 'ownedRelationship').index(element),
            source_ownership_valid=True, complete_implied_binding_and_domain_proof=False))
    if p.is_kind(element, 'Feature'):
        sequence = [r for r in p.refs(element, 'ownedRelationship') if p.is_kind(r, 'FeatureChaining')]
        if sequence:
            chains.append(dict(feature=p.brief(element), length=len(sequence),
                ordered_chaining=[dict(relationship=r, declared_target=p.refs(r, 'chainingFeature'),
                    authored_reference=named_assertion(r)) for r in sequence],
                normative_order='Element.ownedRelationship projected through ownedFeatureChaining',
                complete_semantic_chain_proof=False))
    if p.is_kind(element, 'Type'):
        members = [(r, v) for r, v in p.members(element) if p.is_kind(r, 'FeatureMembership')]
        end_members = [(r, v) for r, v in members if p.scalar(v, 'isEnd') is True]
        parameters = [(r, v) for r, v in members if p.scalar(v, 'direction') is not None and not p.is_kind(r, 'ReturnParameterMembership')]
        results = [(r, v) for r, v in members if p.is_kind(r, 'ReturnParameterMembership')]
        if end_members or parameters or results:
            def seq(xs): return [dict(membership=r, feature=v) for r, v in xs]
            positions.append(dict(owner=p.brief(element), ends=seq(end_members), parameters=seq(parameters),
                results=seq(results), normative_order='ownedMembership projected through FeatureMembership',
                implied_and_inherited_redefinition_closure_complete=False))

documents = {(r['source']['document'], r['source']['sha256']) for r in p.records.values() if r.get('source')}
assert len(documents) == 36
assert not any(r['is_initial'] for r in values)
formal_names = ['featureWithValue', 'deriveFeatureValueValue', 'checkFeatureValuationSpecialization',
    'checkFeatureValueBindingConnector', 'checkFeatureValueBindingConnectorTypeFeaturing',
    'deriveFeatureChainingFeature', 'deriveFeatureFeatureTarget', 'validateFeatureChainingFeatureNotOne',
    'validateFeatureChainingFeaturesNotSelf', 'validateFeatureChainingFeatureConformance',
    'checkFeatureEndRedefinition', 'checkFeatureParameterRedefinition', 'checkFeatureResultRedefinition']
report = dict(format='agentique-v10-source-structural-populations/1', pinned_archive_sha256=digest(p.archive),
    source_documents=[dict(path=path, sha256=sha) for path, sha in sorted(documents)],
    formal=[r for r in inventory['members'] if r['name'] in formal_names],
    feature_values=values, feature_chains=chains, positional_owners=positions,
    counts=dict(feature_values=len(values), default_values=sum(r['is_default'] is True for r in values),
        initial_values=0, feature_chains=len(chains), chain_lengths=dict(collections.Counter(r['length'] for r in chains)),
        positional_owners=len(positions), owned_ends=sum(len(r['ends']) for r in positions),
        owned_nonresult_parameters=sum(len(r['parameters']) for r in positions), owned_results=sum(len(r['results']) for r in positions)),
    source_inventory_complete=True, structural_semantics_complete=False, semantic_publication_accepted=False,
    limitation='These are independently enumerated source facts. Final target resolution, implied results/bindings, featuring domains, inherited order, positional suppression and validation require the expanded-graph audit and complete semantic coverage. No ID sort supplies a semantic position.')
encoded = json.dumps(report, indent=2, ensure_ascii=False)+'\n'
path = OUT / 'source-structural-populations.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print(json.dumps(report['counts']))
