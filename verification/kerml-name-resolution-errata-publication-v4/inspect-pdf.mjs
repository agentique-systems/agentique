import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
const directory = 'verification/kerml-name-resolution-errata-publication-v4';
const result = [];
for (const file of ['KerML.pdf', `${directory}/release/doc/1-Kernel_Modeling_Language.pdf`]) {
  const bytes = fs.readFileSync(file);
  const doc = await getDocument({data: new Uint8Array(bytes), useSystemFonts: true}).promise;
  const pages = [];
  for (const page of [1, 108, 109, 110, 111, 172]) {
    const content = await (await doc.getPage(page)).getTextContent();
    pages.push({page, text: content.items.map(i => i.str + (i.hasEOL ? '\n' : ' ')).join('')});
  }
  result.push({file, sha256: createHash('sha256').update(bytes).digest('hex'), pages});
}
fs.writeFileSync(`${directory}/pdf-clauses.json`, JSON.stringify(result, null, 2)+'\n', {flag:'wx'});
console.log(result.map(r => ({file:r.file, sha256:r.sha256, pages:r.pages.map(p=>p.page)})));
