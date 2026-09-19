import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
const out = 'verification/kerml-semantic-closure-v6';
const patterns = ['RedefinitionEndConformance', 'namingFeature', 'ParameterRedefinition',
  'ResultRedefinition', 'EndRedefinition', 'FeatureChainExpression', 'inheritedMemberships',
  'ResultParameterMembership', 'ownedSpecialization', 'ownedFeatureMembership'];
for (const [label, file] of [['formal', 'KerML.pdf'], ['preliminary', `${out}/release/doc/1-Kernel_Modeling_Language.pdf`]]) {
  const bytes = fs.readFileSync(file);
  const doc = await getDocument({data:new Uint8Array(bytes),useSystemFonts:true}).promise;
  const pages = [];
  for (let page=1; page<=doc.numPages; page++) {
    const content = await (await doc.getPage(page)).getTextContent();
    const text = content.items.map(i=>i.str+(i.hasEOL?'\n':' ')).join('');
    const matches = patterns.filter(p=>text.includes(p));
    if (matches.length) pages.push({page,matches,text});
  }
  fs.writeFileSync(`${out}/${label}-clauses.json`, JSON.stringify({file,sha256:createHash('sha256').update(bytes).digest('hex'),pages},null,2)+'\n',{flag:'wx'});
  console.log(label, pages.map(p=>({page:p.page,matches:p.matches})));
}
