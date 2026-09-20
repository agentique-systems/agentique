"""Offline issue inventory, with exact-name links and explicit related-rule review."""
import hashlib
import json
from pathlib import Path
import re
import sys
from bs4 import BeautifulSoup

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def write(path, value):
    encoded = json.dumps(value, indent=2, ensure_ascii=False)+'\n'
    if '--check' in sys.argv:
        assert path.read_text(encoding='utf8') == encoded, str(path)
    else:
        path.write_text(encoded, encoding='utf8', newline='\n')


raw = OUT/'issues/KerML-all.html'
soup = BeautifulSoup(raw.read_bytes(), 'html.parser')
issues = []
for article in soup.select('article.issue-description'):
    text = article.get_text(' ', strip=True)
    key = re.search(r'Key:\s*(\S+)', text).group(1)
    status = re.search(r'Status:\s*(open|closed)', text).group(1)
    issues.append(dict(key=key, title=article.h2.get_text(' ', strip=True), status=status,
        url=f'https://issues.omg.org/issues/{key}', anchor=article['id'], text=text))
assert len(issues) == 410 and len({i['key'] for i in issues}) == 410
write(OUT/'issue-inventory.json', dict(format='agentique-kerml-issue-inventory/1',
    source=str(raw.relative_to(ROOT).as_posix()), sha256=sha(raw), issues=issues))

inventory_path = ROOT/'verification/kerml-semantic-closure-v7/validation-coverage.json'
inventory = json.loads(inventory_path.read_text())
authorized = {'KERML11-81':1,'KERML11-140':2,'KERML11-76':3,'KERML11-68':4,
    'KERML11-205':5,'KERML11-206':5,'KERML11-207':5,'KERML11-145':6}
# These links supplement lexical matching; no correction is authorized by a link.
related = {
    'KERML11-8':['deriveConnectorDefaultFeaturingType','deriveFeatureFeaturingType','checkFeatureReferenceExpressionResultSpecialization'],
    'KERML11-145':['validateSubsettingFeaturingTypes','checkConnectorTypeFeaturing','checkFeatureValueBindingConnector'],
    'KERML11-182':['deriveFeatureReferenceExpressionReferent','validateFeatureReferenceExpressionReferentIsFeature'],
    'KERML11-210':['validateSubsettingFeaturingTypes','checkConnectorTypeFeaturing','checkFeatureReferenceExpressionBindingConnector'],
    'KERML11-191':['deriveTypeFeature','deriveTypeInheritedFeature','deriveTypeInheritedMembership'],
    'KERML11-76':['validateNamespaceDistinguishibility'],
    'KERML11-140':['validateRedefinitionFeaturingTypes'],
    'KERML11-69':['checkIndexExpressionResultSpecialization'],
    'KERML11-3':['checkFeatureOwnedCrossFeatureTypeFeaturing','checkMultiplicityTypeFeaturing','validateFeatureMultiplicityDomain'],
    'KERML11-1':['deriveFeatureOwnedCrossSubsetting','deriveFeatureCrossFeature','checkFeatureOwnedCrossFeatureSpecialization'],
    'KERML11-12':['derivePackageFilterCondition','deriveNamespaceImportedMembership'],
    'KERML11-75':['deriveNamespaceImportedMembership','deriveNamespaceMembers','deriveMembershipImportImportedElement','deriveNamespaceImportImportedElement'],
    'KERML11-72':['deriveTypeInheritedMembership','deriveTypeInheritedFeature','deriveTypeFeatureMembership','validateNamespaceDistinguishibility'],
    'KERML11-50':['validateEndFeatureMembershipIsEnd','validateParameterMembershipOwningType','deriveTypeOwnedEndFeature'],
    'KERML11-35':['deriveFeatureType','validateSubsettingFeaturingTypes'],
    'KERML11-38':['validateRedefinitionDirectionConformance','validateRedefinitionEndConformance','validateRedefinitionFeaturingTypes'],
    'KERML11-34':['deriveTypeMultiplicity','validateTypeOwnedMultiplicity'],
    'KERML11-14':['validateClassifierMultiplicityDomain'],
    'KERML11-15':['validateFeatureMultiplicityDomain','checkMultiplicityTypeFeaturing'],
    'KERML11-37':['validateParameterMembershipParameterDirection','deriveTypeDirectedFeature'],
    'KERML11-197':['deriveFlowFlowEnd','deriveFlowPayloadFeature','deriveFlowPayloadType','checkFeatureFlowFeatureRedefinition','checkPayloadFeatureRedefinition'],
}
editorial = {'KERML11-70','KERML11-6','KERML11-7','KERML11-11','KERML11-74',
    'KERML11-91','KERML11-111','KERML11-198','KERML11-180','KERML11-33'}
execution_or_description = {'KERML11-31','KERML11-34','KERML11-21','KERML11-14','KERML11-15','KERML11-37'}
structural = set(related) | {'KERML11-4','KERML11-78','KERML11-82','KERML11-90','KERML11-96','KERML11-183'}
def assessment(issue):
    key=issue['key']
    if key in editorial:
        return False, 'Editorial body/name or guard defect; clearer existing prose establishes intent. No additional canonical graph correction adopted.'
    if key in execution_or_description:
        return False, 'Discusses mathematical/value/timing meaning or descriptive mismatch. No different required canonical structure is established by this preflight; structural slots and relationships remain required.'
    if key in authorized or key in structural:
        return True, 'Touches structural identities, relationships or applicability. This flag records a structural dispute, not that its proposed correction is adopted or needed for this corpus.'
    if issue['status']=='closed':
        return None, 'Historical closed issue retained as authority lineage. Whether its historical resolution changed graph structure is not independently re-adjudicated here; pinned KerML 1.0 remains authoritative.'
    raise AssertionError(f'Unreviewed open named-rule match: {key}')

def corpus_review(key,rule_name):
    if key=='KERML11-8': return True, 'Independently reproduced after required inference; feature-reference-authority-conflict.json.'
    if key=='KERML11-145':
        if rule_name=='checkSelectExpressionResultSpecialization': return False, 'Zero SelectExpression population in the pinned construction and reference XMI; source applicability only, not a final expansion completeness claim.'
        if rule_name=='checkFunctionResultBindingConnector': return True, 'Exact ControlFunctions dot Function witness independently reproduced; authority-matrix.json.'
        return None, 'Per-rule source and reference witnesses retained in authority-matrix.json; a complete five-rule corpus conformance audit is not claimed at the Gate 1 stop.'
    if key in authorized: return True, 'Historical reviewed correction witnesses retained in the frozen v1-v5 evidence.'
    if key=='KERML11-183': return False, 'The inventory has zero ElementFilterMembership population; example-only disputed case is not present in the current construction.'
    if key=='KERML11-69': return None, '23 IndexExpressions require guard applicability review; Collection-vs-Array pilot difference recorded in authority-matrix.json. No correction is adopted.'
    if key=='KERML11-72': return None, 'Inherited-member/Triggers diagnostics require complete inference before attributing them to this issue. They remain ordinary implementation work.'
    if key=='KERML11-191': return None, 'Imported memberships of supertypes require per-population review; existing incomplete queries are not conflict evidence.'
    if key=='KERML11-197': return None, 'Longhand Flow populations require inference review; no contradictory required result independently established.'
    return None, 'Future-risk metadata at the KLCV8-F-001 preflight stop; no independently established contradictory pinned result.'
rows = []
for rule in inventory['constraints']:
    links = []
    for issue in issues:
        direct = rule['name'].casefold() in issue['text'].casefold()
        semantic = rule['name'] in related.get(issue['key'],[])
        if not (direct or semantic): continue
        structural_change, review = assessment(issue)
        exercised, corpus_evidence = corpus_review(issue['key'],rule['name'])
        links.append(dict(key=issue['key'], status=issue['status'], url=issue['url'],
            evidence_anchor=issue['anchor'], match='formal-rule-name' if direct else 'reviewed-related-rule',
            changes_canonical_structural_semantics=structural_change,
            canonical_change_review=review,
            operational_correction_authorized=issue['key'] in authorized,
            earliest_authorized_profile=authorized.get(issue['key']),
            pinned_corpus_exercises_disputed_case=exercised,pinned_disputed_case=corpus_evidence))
    status=rule['implementation_status']
    if rule['name']=='checkFunctionResultBindingConnector': status='NotYetImplemented: v6 correction authorized; no longer an unauthorized authority conflict'
    if rule['name']=='checkFeatureReferenceExpressionBindingConnector': status='NewAuthorityConflict: KLCV8-F-001'
    rows.append(dict(external_rule_id=rule['external_id'], rule=rule['name'], owning_metaclass=rule['owning_metaclass'],
        current_implementation_status=status,historical_v7_implementation_status=rule['implementation_status'],
        corpus_applicability=rule['corpus_applicability'], baseline_population=rule['owning_metaclass_population'],
        known_omg_issues=links, authority_review='Named rule cross-reference plus explicitly recorded related-rule review; absence of a named match is not proof that no issue can affect the rule.'))
assert len(rows)==258
write(ROOT/'standards/kerml-1.0-constraint-authority-map.json', dict(
    format='agentique-kerml-constraint-authority-map/1', specification='KerML 1.0',
    tracker=dict(url='https://issues.omg.org/issues/spec/KerML?view=ALL', retained=str(raw.relative_to(ROOT).as_posix()),sha256=sha(raw),issues=len(issues)),
    implementation_inventory=dict(path=str(inventory_path.relative_to(ROOT).as_posix()),sha256=sha(inventory_path)),
    scope='Authority preflight, not validation acceptance or authorization of additional corrections.',
    v6_authorization='Exactly the five KERML11-145 families are authorized by the task. Authorization is not an implementation claim.',
    review_boundary='All 258 rules cross-referenced; 410 tracker records retained. Known pending topics are exposed early, not silently authorized. Null applicability is explicitly unproven risk metadata at the independent authority stop, not a closed publication coverage gate.',
    constraints=rows))
write(OUT/'open-issue-preflight.json', dict(format='agentique-kerml-open-issue-preflight/1',
    authority_stop='KLCV8-F-001',publication_gate_closed=False,
    issues=[dict(key=i['key'],title=i['title'],url=i['url'],status=i['status'],
        linked_rules=[r['external_rule_id'] for r in rows if any(link['key']==i['key'] for link in r['known_omg_issues'])],
        review=('Named or related structural-rule cross-reference; details in constraint authority map.' if any(any(link['key']==i['key'] for link in r['known_omg_issues']) for r in rows) else
            'No rule-name or recorded related-rule match in this inventory; retained as additional syntax/library/mathematical authority risk, not a non-applicability proof.'))
        for i in issues if i['status']=='open']))
print(f'{len(rows)} formal constraints cross-referenced against {len(issues)} official issue records')
