import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
const file = 'KerML.pdf';
const bytes = fs.readFileSync(file);
const doc = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: true }).promise;
const patterns = ['checkFeatureParameterRedefinition', 'checkFeatureResultRedefinition',
  'validateRedefinitionEndConformance', 'sourceTargetFeature', 'src.f', 'namingFeature',
  'checkFeatureChainExpression', 'transitionLinkTarget'];
const pages = [];
for (let page = 1; page <= doc.numPages; page++) {
  const content = await (await doc.getPage(page)).getTextContent();
  const text = content.items.map(i => i.str + (i.hasEOL ? '\n' : ' ')).join('');
  const matches = patterns.filter(pattern => text.includes(pattern));
  if (matches.length) pages.push({ page, matches, text });
}
fs.writeFileSync('verification/kerml-library-content-errata-publication-v5/formal-clauses.json',
  JSON.stringify({file, sha256:createHash('sha256').update(bytes).digest('hex'), pages},null,2)+'\n',{flag:'wx'});
console.log(pages.map(p => ({page:p.page,matches:p.matches})));
