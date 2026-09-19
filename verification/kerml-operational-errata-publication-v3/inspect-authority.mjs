import fs from "node:fs";
import { createHash } from "node:crypto";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";

const directory = "verification/kerml-operational-errata-publication-v3";
const result = [];
for (const file of ["KerML.pdf", `${directory}/revised-kerml.pdf`]) {
  const bytes = fs.readFileSync(file);
  const document = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: true }).promise;
  const selected = [];
  for (let n = 1; n <= document.numPages; n++) {
    const content = await (await document.getPage(n)).getTextContent();
    const text = content.items.map(item => item.str + (item.hasEOL ? "\n" : " ")).join("");
    if (n === 1 || text.includes("any of these, then the overall resolution fails") ||
        (file === "KerML.pdf" && [172, 208, 249, 264].includes(n))) selected.push({ pdf_page: n, text });
  }
  result.push({ file, sha256: createHash("sha256").update(bytes).digest("hex"), pages: document.numPages, selected });
}
const encoded = JSON.stringify(result, null, 2) + "\n";
const output = `${directory}/inspected-authority.json`;
if (process.argv.includes("--check")) {
  if (fs.readFileSync(output, "utf8") !== encoded) throw new Error("Stale authority inspection");
} else fs.writeFileSync(output, encoded, { flag: "wx" });
console.log(result.map(r => ({ file: r.file, sha256: r.sha256, pages: r.pages, inspectedPages: r.selected.map(p => p.pdf_page) })));
