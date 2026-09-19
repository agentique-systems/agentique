import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
const file = 'KerML.pdf';
const bytes = fs.readFileSync(file);
const doc = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: true }).promise;
const expressions = [
  'checkAssociationBinarySpecialization', 'checkFeatureEndRedefinition',
  'deriveAssociationAssociationEnd', 'deriveTypeEndFeature',
  'removeRedefinedFeatures', 'inheritableMemberships', 'supertypes()',
  'validateRedefinition', 'validateExpressionResultParameterMembership',
];
const pages = [];
for (let page = 1; page <= doc.numPages; page++) {
  const content = await (await doc.getPage(page)).getTextContent();
  const text = content.items.map(i => i.str + (i.hasEOL ? '\n' : ' ')).join('');
  const matches = expressions.filter(e => text.includes(e));
  if (matches.length) pages.push({ page, matches, text });
}
const out = { file, sha256: createHash('sha256').update(bytes).digest('hex'), pages };
fs.writeFileSync('verification/kerml-name-resolution-errata-publication-v4/structural-authority.json', JSON.stringify(out, null, 2) + '\n', { flag: 'wx' });
console.log(pages.map(p => ({ page: p.page, matches: p.matches })));
