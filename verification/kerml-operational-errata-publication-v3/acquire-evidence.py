"""One-time evidence acquisition; never invoked by builds or verification."""
import datetime
import hashlib
import json
from pathlib import Path
import re
import urllib.request
import xml.etree.ElementTree as ET

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
records = []


def acquire(name, url):
    with urllib.request.urlopen(url, timeout=45) as response:
        data = response.read()
        record = dict(path=name, url=url, resolved_url=response.url,
                      retrieved_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
    with (OUT / name).open('xb') as stream:
        stream.write(data)
    records.append(record)
    return data


issue = acquire('KERML11-81.html', 'https://issues.omg.org/issues/KERML11-81')
links = re.findall(r'href="([^"]+)"', issue.decode())
task = next(link for link in links if '/task-force/' in link)
if task.startswith('/'):
    task = 'https://issues.omg.org' + task
acquire('KERML11-current.html', task)
api = 'https://api.github.com/repos/Systems-Modeling/SysML-v2-Release'
commit = json.loads(acquire('release-head.json', api + '/commits/master'))['sha']
acquire('release-latest.json', api + '/releases/latest')
acquire('release-2026-05.json', api + '/releases/tags/2026-05')
tree = json.loads(acquire('release-tree.json', api + '/git/trees/' + commit + '?recursive=1'))
acquire('revised-kerml.pdf', 'https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/'
        + commit + '/doc/1-Kernel_Modeling_Language.pdf')
revised = [v['path'] for v in tree['tree'] if v['path'].lower().endswith('/kerml.xmi')]
for i, path in enumerate(revised):
    acquire(f'revised-kerml-{i}.xmi', 'https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/' + commit + '/' + path)
local = ['standards/normative/kerml-1.0/KerML.xmi', 'KerML.pdf',
         'docs/kerml-standard-library-publication-authority-conflict.md',
         'crates/kerml-semantics/tests/participant_contract.rs']
local += [v['path'] for v in json.loads((ROOT / 'standards/baseline-lock.json').read_bytes())['artifacts']
          if v['path'].endswith('.kpar')]
preserved = [dict(path=p, sha256=hashlib.sha256((ROOT/p).read_bytes()).hexdigest(),
                  bytes=(ROOT/p).stat().st_size) for p in local]
raw = (ROOT / local[0]).read_bytes()
xmi = ET.fromstring(raw)
XMI = '{http://www.omg.org/spec/XMI/20131001}'
if not any(XMI+'id' in e.attrib for e in xmi.iter()):
    XMI = '{http://www.omg.org/spec/XMI/20161101}'
associations = [e for e in xmi.iter() if e.get(XMI+'type') == 'uml:Association'
                and any(c.tag == 'ownedEnd' and c.get('name') == 'participantFeature' for c in e)]
assert len(associations) == 2
closure = {e.get(XMI+'id') for a in associations for e in a.iter() if e.get(XMI+'id')}
parents = {c:p for p in xmi.iter() for c in p}
incoming = []
for e in xmi.iter():
    for key, value in e.attrib.items():
        if key == XMI+'id':
            continue
        for target in value.split():
            if target in closure:
                p = e
                while p is not None and p.get(XMI+'id') not in closure:
                    p = parents.get(p)
                incoming.append(dict(tag=e.tag, attribute=key, target=target, inside_closure=p is not None))
assert all(v['inside_closure'] for v in incoming), incoming
report = dict(format='agentique-errata-authority-evidence/1', reviewed_at='2026-09-19',
              issue_status='open', disposition='Agentique operational interpretation; not an adopted KerML 1.0 erratum',
              remote=records, preserved=preserved, current_release_commit=commit,
              revised_abstract_syntax_paths=revised,
              revised_abstract_syntax_availability='No KerML.xmi in the current release tree; revised PDF pinned as corroboration',
              published_association_xml=[ET.tostring(a, encoding='unicode') for a in associations],
              xml_deletion_closure=sorted(closure), incoming_references=incoming)
with (OUT/'authority-evidence.json').open('x', encoding='utf-8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(commit=commit, xml_closure=sorted(closure), incoming_count=len(incoming),
                     external_incoming_count=sum(not v['inside_closure'] for v in incoming))))
