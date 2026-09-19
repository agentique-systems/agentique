import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {getDocument} from 'pdfjs-dist/legacy/build/pdf.mjs';
const out='verification/kerml-semantic-closure-v6';
const artifacts=[];
for (const file of ['KerML.pdf',`${out}/release/doc/1-Kernel_Modeling_Language.pdf`]) {
  const bytes=fs.readFileSync(file);
  const pdf=await getDocument({data:new Uint8Array(bytes),useSystemFonts:true}).promise;
  const pages=[];
  let continuation=false;
  for(let page=1;page<=pdf.numPages;page++) {
    const content=await(await pdf.getPage(page)).getTextContent();
    const text=content.items.map(i=>i.str+(i.hasEOL?'\n':' ')).join('');
    const matched=['checkFeatureSubobjectSpecialization','checkStepSubperformanceSpecialization',
        'checkStepEnclosedPerformanceSpecialization','specializesFromLibrary(libraryTypeName',
        'specializes(supertype'].some(p=>text.includes(p));
    if(matched||continuation) pages.push({page,text});
    continuation=matched;
  }
  artifacts.push({file,sha256:createHash('sha256').update(bytes).digest('hex'),pages});
  console.log(file,pages.map(p=>p.page));
}
fs.writeFileSync(`${out}/subobject-formal-clauses-v2.json`,JSON.stringify(artifacts,null,2)+'\n',{flag:'wx'});
