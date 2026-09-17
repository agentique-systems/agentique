import fs from "node:fs";
import path from "node:path";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { unzipSync } from "fflate";
import { root, hash, safePath } from "./extract.mjs";

const priorLock = path.join(root, "standards/lock.json");
if (
  fs.existsSync(priorLock) &&
  JSON.parse(fs.readFileSync(priorLock, "utf8")).library_publication
)
  throw Error(
    "A corrective library release is pinned. Original baseline is preserved in standards/baseline-lock.json; this acquisition command cannot replace the active lock.",
  );
if (fs.existsSync(priorLock)) {
  for (const artifact of JSON.parse(fs.readFileSync(priorLock, "utf8"))
    .artifacts) {
    if (
      hash(fs.readFileSync(safePath(root, artifact.path))) !== artifact.sha256
    )
      throw Error(
        `Pinned artifact modified: ${artifact.path}. Refusing to repin altered bytes.`,
      );
  }
}

fs.mkdirSync(path.join(root, "standards/artifacts"), { recursive: true });
fs.mkdirSync(path.join(root, ".cache"), { recursive: true });
const references = JSON.parse(
  fs.readFileSync(path.join(root, "standards.references.json")),
).sources;
const local = {
  "kerml-spec": "KerML.pdf",
  "sysml-spec": "SysML.pdf",
  "api-spec": "SysAPI.pdf",
};
const records = [];
for (const ref of references) {
  const filename =
    local[ref.id] ?? `standards/artifacts/${ref.source.split("/").at(-1)}`;
  const target = safePath(root, filename);
  if (!fs.existsSync(target)) {
    const response = await fetch(ref.source);
    if (!response.ok) throw Error(`${ref.source}: ${response.status}`);
    fs.writeFileSync(target, Buffer.from(await response.arrayBuffer()), {
      flag: "wx",
    });
  }
  const bytes = fs.readFileSync(target);
  const record = {
    ...ref,
    path: filename,
    sha256: hash(bytes),
    bytes: bytes.length,
    artifact_status: "bytes_acquired_and_hashed",
  };
  if (filename.endsWith(".pdf")) {
    const pdf = await getDocument({
      data: new Uint8Array(bytes),
      useSystemFonts: true,
    }).promise;
    let text = "";
    for (let i = 1; i <= pdf.numPages; i++) {
      const content = await (await pdf.getPage(i)).getTextContent();
      text +=
        `\n--- PDF PAGE ${i} ---\n` +
        content.items.map((x) => x.str + (x.hasEOL ? "\n" : " ")).join("");
    }
    fs.writeFileSync(path.join(root, `.cache/${filename}.txt`), text);
    record.title_page = text
      .slice(0, text.indexOf("--- PDF PAGE 2 ---"))
      .trim();
    record.pages = pdf.numPages;
    console.log(record.title_page);
  }
  if (filename.endsWith(".kpar")) {
    const entries = unzipSync(bytes);
    record.entries = Object.entries(entries).map(([name, data]) => ({
      path: name,
      sha256: hash(data),
      bytes: data.length,
    }));
    record.metadata = Object.fromEntries(
      Object.entries(entries)
        .filter(([name]) => name.endsWith(".json"))
        .map(([name, data]) => [
          name,
          JSON.parse(new TextDecoder().decode(data)),
        ]),
    );
    for (const [name, data] of Object.entries(entries)) {
      if (name.endsWith("/")) continue;
      const output = safePath(
        root,
        `standards/libraries/${path.basename(filename, ".kpar")}/${name}`,
      );
      fs.mkdirSync(path.dirname(output), { recursive: true });
      if (fs.existsSync(output) && hash(fs.readFileSync(output)) !== hash(data))
        throw Error(`Modified library: ${output}`);
      if (!fs.existsSync(output))
        fs.writeFileSync(output, data, { flag: "wx" });
    }
    console.log(ref.id, JSON.stringify(record.metadata));
  }
  records.push(record);
}
const extraUrl =
  "https://www.omg.org/spec/KerML/20250201/KerML-Model-Interchange.json";
const extraPath = path.join(
  root,
  "standards/artifacts/KerML-Model-Interchange.json",
);
if (!fs.existsSync(extraPath)) {
  const response = await fetch(extraUrl);
  if (!response.ok) throw Error(response.status);
  fs.writeFileSync(extraPath, Buffer.from(await response.arrayBuffer()), {
    flag: "wx",
  });
}
records.push({
  id: "interchange-schema",
  source: extraUrl,
  path: "standards/artifacts/KerML-Model-Interchange.json",
  sha256: hash(fs.readFileSync(extraPath)),
});
fs.writeFileSync(
  path.join(root, "standards/lock.json"),
  JSON.stringify(
    { format: "agentique-standards-lock/0.1", artifacts: records },
    null,
    2,
  ) + "\n",
);
