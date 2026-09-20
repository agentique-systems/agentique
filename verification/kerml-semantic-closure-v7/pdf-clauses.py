"""Extract the pinned PDF pages used in the target/domain authority review."""
import hashlib
import json
from pathlib import Path
import sys
from pypdf import PdfReader

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
names=[r['rule'] for r in json.loads((OUT/'authority-matrix.json').read_text())['constraints']]
names+=['checkFunctionResultBindingConnector','checkConnectorTypeFeaturing',
        'checkFeatureFeatureMembershipTypeFeaturing','isFeaturingType',
        'isFeaturedWithin','isCompatibleWith']
source=ROOT/'KerML.pdf'
pages=[]
for index,page in enumerate(PdfReader(source).pages):
    text=page.extract_text()
    matches=[name for name in names if name in text]
    if matches: pages.append(dict(page=index+1,matches=matches,text=text))
report=dict(path='KerML.pdf',sha256=hashlib.sha256(source.read_bytes()).hexdigest(),pages=pages)
encoded=json.dumps(report,indent=2)+'\n'
path=OUT/'formal-pdf-clauses.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print('Pinned PDF pages captured:',len(pages))
