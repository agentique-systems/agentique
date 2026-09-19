import fs from "node:fs";
import { createHash } from "node:crypto";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";

const bytes = fs.readFileSync("KerML.pdf");
const document = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: true }).promise;
let text = `Pinned KerML.pdf SHA256: ${createHash("sha256").update(bytes).digest("hex")}\n`;
for (let page = 1; page <= document.numPages; page++) {
  const content = await (await document.getPage(page)).getTextContent();
  text += `\n=== PDF page ${page} ===\n`;
  text += content.items.map(item => item.str + (item.hasEOL ? "\n" : " ")).join("") + "\n";
}
const output = "verification/kerml-standard-library-publication-v2/kerml-authority-extract.txt";
if (process.argv.includes("--check")) {
  if (fs.readFileSync(output, "utf8") !== text) throw new Error("Stale PDF extraction");
} else {
  fs.writeFileSync(output, text, { flag: "wx" });
}
console.log(`Extracted ${document.numPages} pages from unchanged pinned PDF.`);
