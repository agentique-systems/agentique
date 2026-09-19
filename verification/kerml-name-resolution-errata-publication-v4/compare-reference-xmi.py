"""Independent non-normative XMI oracle. No Agentique IDs or resolver in extraction."""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.parse
import xml.etree.ElementTree as ET

OUT = Path(__file__).resolve().parent
XSI = '{http://www.w3.org/2001/XMLSchema-instance}type'
XID = '{http://www.omg.org/XMI}id'
DIRECTORY = OUT/'release/sysml.library.xmi/Kernel Libraries/Kernel Semantic Library'
trees = {p.name: (p, ET.parse(p).getroot()) for p in DIRECTORY.glob('*.kermlx')}
parents = {c: p for _, root in trees.values() for p in root.iter() for c in p}
files = {e: name for name, (_, root) in trees.items() for e in root.iter()}
ids = {(name, e.get(XID)): e for name, (_, root) in trees.items() for e in root.iter() if e.get(XID)}


def target(element, property):
    local = element.get(property)
    if local:
        return ids.get((files[element], local)), local
    href = next((c.get('href') for c in element if c.tag == property), None)
    if href and '#' in href:
        file, identifier = href.split('#', 1)
        return ids.get((Path(urllib.parse.unquote(file)).name, identifier)), href
    return None, href


def qname(element, seen=frozenset()):
    if element in seen:
        return '<cycle>'
    names = []
    while element is not None:
        name = element.get('declaredName')
        if not name and element.tag == 'ownedRelatedElement':
            for relationship in element:
                if relationship.get(XSI) == 'sysml:Redefinition':
                    value, _ = target(relationship, 'redefinedFeature')
                    if value is not None:
                        name = qname(value, seen | {element}).split('::')[-1]
                    break
        if name:
            names.append(name)
        element = parents.get(element)
    return '::'.join(reversed(names))


def extract():
    rows = []
    for file, (path, root) in sorted(trees.items()):
        for e in root.iter():
            if e.get(XSI) != 'sysml:Redefinition':
                continue
            value, identifier = target(e, 'redefinedFeature')
            source, source_identifier = target(e, 'redefiningFeature')
            if source is None:
                source = parents[e]
            rows.append(dict(artifact=str(path.relative_to(OUT)).replace('\\', '/'),
                             sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                             relationship_id=e.get(XID), source_id=source_identifier,
                             source_declaration=qname(source), target_id=identifier,
                             target=qname(value) if value is not None else None,
                             authority_status='non-normative reference implementation interchange',
                             xml=ET.tostring(e, encoding='unicode')))
    return rows


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--audit')
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    oracle = extract()
    report = dict(format='agentique-reference-xmi-comparison/1', oracle=oracle)
    if args.audit:
        audit = json.loads(Path(args.audit).read_text())
        report['agentique_input'] = dict(path=args.audit,
            sha256=hashlib.sha256(Path(args.audit).read_bytes()).hexdigest(),
            profile_id=audit['baseline_profile'], rule_set_version=audit['rule_version'],
            library_set=audit['library_set'], accepted_semantic_publication=audit['publication_accepted'])
        rows = []
        by_source = {}
        for row in oracle:
            by_source.setdefault(row['source_declaration'], []).append(row)
        for source in audit['redefinition_resolutions']:
            matches = by_source.get(source['source_declaration'], [])
            if not matches:
                continue
            expected = {m['target'] for m in matches if m['target']}
            actual = {t['path'] for t in source['targets']}
            # Several explicit Redefinitions can be owned by one source feature.
            # Each distinct assertion must select one of its independent XMI targets.
            rows.append(dict(source_declaration=source['source_declaration'],
                             source_reference=source['source'],
                             source_range=source['source_range'],
                             agentique_target=sorted(actual), reference_implementation_targets=sorted(expected),
                             target_match=len(actual) == 1 and actual <= expected,
                             resolution_complete=source['result']['completeness'] == 'Complete',
                             match=len(actual) == 1 and actual <= expected and source['result']['completeness'] == 'Complete',
                             profile_id=audit['baseline_profile'],
                             completeness=source['result']['completeness'],
                             authority_status='non-normative operational oracle',
                             xmi_relationships=[m['relationship_id'] for m in matches]))
        report['comparisons'] = rows
        groups = []
        for declaration in sorted({r['source_declaration'] for r in rows}):
            assertions = [r for r in rows if r['source_declaration'] == declaration]
            expected = {m['target'] for m in by_source[declaration] if m['target']}
            actual = {t for r in assertions for t in r['agentique_target']}
            groups.append(dict(source_declaration=declaration,
                               agentique_targets=sorted(actual), reference_implementation_targets=sorted(expected),
                               agentique_assertion_count=len(assertions), reference_assertion_count=len(by_source[declaration]),
                               target_match=actual == expected,
                               resolution_complete=all(r['resolution_complete'] for r in assertions),
                               match=actual == expected and all(r['resolution_complete'] for r in assertions),
                               authority_status='non-normative operational oracle; complete target-set comparison'))
        report['source_target_sets'] = groups
        exact = [r for r in rows if any(p in r['source_declaration'] for p in
                 ['beforeTimeSlice::monitoredFeature', 'afterSnapshot::monitoredFeature'])]
        report['exact_kerml11_140_matches'] = len(exact) == 2 and all(r['match'] for r in exact)
        report['unmatched_comparisons'] = sum(not r['match'] for r in rows)
        report['unmatched_target_sets'] = sum(not r['match'] for r in groups)
        report['target_identity_mismatches'] = sum(not r['target_match'] for r in rows)
        report['incomplete_reference_answers'] = sum(not r['resolution_complete'] for r in rows)
    output = Path(args.output)
    if output.exists():
        assert json.loads(output.read_bytes()) == report, 'Existing comparison differs; use a fresh output'
    else:
        with output.open('x', encoding='utf-8') as stream:
            json.dump(report, stream, indent=2)
            stream.write('\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ['oracle', 'comparisons', 'source_target_sets']}))
    print('Oracle references:', len(oracle))
    if args.audit and (not report['exact_kerml11_140_matches'] or report['unmatched_comparisons'] or report['unmatched_target_sets']):
        raise SystemExit(1)
