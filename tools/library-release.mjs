// Select the official corrective library publication without modifying baseline bytes.
import fs from "node:fs";
import path from "node:path";
import { unzipSync } from "fflate";
import { root, hash, safePath } from "./extract.mjs";
const commit = "9baca5908ca28b53da085de69336fde48420ea8f";
const baselinePath = path.join(root, "standards/baseline-lock.json");
if (!fs.existsSync(baselinePath))
  fs.copyFileSync(
    path.join(root, "standards/lock.json"),
    baselinePath,
    fs.constants.COPYFILE_EXCL,
  );
const baseline = JSON.parse(fs.readFileSync(baselinePath, "utf8"));
const names = {
  "kerml-semantic-library": "Kernel_Semantic_Library-1.0.0",
  "kerml-data-library": "Kernel_Data_Type_Library-1.0.0",
  "kerml-function-library": "Kernel_Function_Library-1.0.0",
  "sysml-systems-library": "SysML_Systems_Library-2.0.0",
};
const artifacts = [];
for (const original of baseline.artifacts) {
  if (!names[original.id]) {
    artifacts.push(original);
    continue;
  }
  const name = names[original.id],
    file = `standards/artifacts/2026-04/${name}.kpar`;
  const source = `https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/${commit}/sysml.library.kpar/${name}.kpar`;
  const target = safePath(root, file);
  if (!fs.existsSync(target)) {
    const r = await fetch(source);
    if (!r.ok) throw Error(r.status);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, Buffer.from(await r.arrayBuffer()), {
      flag: "wx",
    });
  }
  const bytes = fs.readFileSync(target),
    entries = unzipSync(bytes),
    extract_root = `standards/libraries-2026-04/${name}`;
  const prior = JSON.parse(
    fs.readFileSync(path.join(root, "standards/lock.json"), "utf8"),
  );
  const existing = prior.artifacts.find((a) => a.path === file);
  if (existing && existing.sha256 !== hash(bytes))
    throw Error(`Pinned artifact modified: ${file}`);
  for (const [entry, data] of Object.entries(entries)) {
    if (entry.endsWith("/")) continue;
    const out = safePath(root, `${extract_root}/${entry}`);
    fs.mkdirSync(path.dirname(out), { recursive: true });
    if (fs.existsSync(out) && hash(fs.readFileSync(out)) !== hash(data))
      throw Error(`Modified library ${out}`);
    if (!fs.existsSync(out)) fs.writeFileSync(out, data, { flag: "wx" });
  }
  artifacts.push({
    ...original,
    path: file,
    source,
    canonical_resource: original.source,
    publication: "Official SysML v2 Release 2026-04",
    commit,
    extract_root,
    sha256: hash(bytes),
    bytes: bytes.length,
    entries: Object.entries(entries).map(([p, b]) => ({
      path: p,
      sha256: hash(b),
      bytes: b.length,
    })),
    metadata: Object.fromEntries(
      Object.entries(entries)
        .filter(([p]) => p.endsWith(".json"))
        .map(([p, b]) => [p, JSON.parse(new TextDecoder().decode(b))]),
    ),
  });
}
fs.writeFileSync(
  path.join(root, "standards/lock.json"),
  JSON.stringify(
    {
      format: baseline.format,
      library_publication: "2026-04",
      library_source_root: "standards/libraries-2026-04",
      baseline_lock: "standards/baseline-lock.json",
      selection_basis:
        "Official corrections to StateTransitionAction payload direction and TransitionPerformance receiver binding; originals retained. See docs/standards-discrepancies.md.",
      artifacts,
    },
    null,
    2,
  ) + "\n",
);
console.log("Pinned official 2026-04 libraries; original baseline preserved.");
