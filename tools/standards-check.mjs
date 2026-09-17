import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { unzipSync } from "fflate";
import { root, hash, safePath } from "./extract.mjs";
import { verifyNormativeArtifacts } from "./normative-artifacts.mjs";
import { verifySysmlArtifacts } from "./sysml-artifacts.mjs";
const lock = JSON.parse(
  fs.readFileSync(path.join(root, "standards/lock.json"), "utf8"),
);
const projects = new Map(),
  discrepancies = [],
  dependencies = [];
let sourceFiles = 0,
  totalBytes = 0;
if (lock.baseline_lock) {
  const baseline = JSON.parse(
    fs.readFileSync(safePath(root, lock.baseline_lock), "utf8"),
  );
  for (const artifact of baseline.artifacts) {
    assert.equal(
      hash(fs.readFileSync(safePath(root, artifact.path))),
      artifact.sha256,
      artifact.path,
    );
    for (const entry of artifact.entries ?? [])
      if (!entry.path.endsWith("/")) {
        const target = `standards/libraries/${path.basename(artifact.path, ".kpar")}/${entry.path}`;
        assert.equal(
          hash(fs.readFileSync(safePath(root, target))),
          entry.sha256,
          target,
        );
      }
  }
}
for (const artifact of lock.artifacts) {
  const bytes = fs.readFileSync(safePath(root, artifact.path));
  assert.equal(hash(bytes), artifact.sha256, artifact.path);
  totalBytes += bytes.length;
  if (!artifact.path.endsWith(".kpar")) continue;
  const entries = unzipSync(bytes);
  const metadata = Object.entries(entries).find(
    ([p]) => p.endsWith("/.project.json") || p === ".project.json",
  );
  assert(metadata, artifact.id);
  const prefix = metadata[0].slice(0, -".project.json".length),
    project = JSON.parse(new TextDecoder().decode(metadata[1]));
  projects.set(artifact.canonical_resource ?? artifact.source, {
    artifact,
    project,
  });
  if (prefix)
    discrepancies.push({
      artifact: artifact.id,
      code: "wrapped_project_root",
      actual: prefix,
      basis: "KerML 10.3 describes top-level project metadata",
      handling:
        "Original bytes preserved; baseline extracts this wrapper. Agentique exports top-level metadata.",
    });
  const meta = JSON.parse(
    new TextDecoder().decode(entries[prefix + ".meta.json"]),
  );
  for (const [name, file] of Object.entries(meta.index)) {
    if (!entries[prefix + file])
      discrepancies.push({
        artifact: artifact.id,
        code: "index_missing_file",
        name,
        file,
        handling:
          "Original metadata preserved; declaration index reads actual pinned textual files.",
      });
  }
  for (const entry of artifact.entries) {
    const data = entries[entry.path];
    assert(data, entry.path);
    assert.equal(hash(data), entry.sha256, entry.path);
    if (entry.path.endsWith("/")) continue;
    const target = `${artifact.extract_root ?? `standards/libraries/${path.basename(artifact.path, ".kpar")}`}/${entry.path}`;
    assert.equal(
      hash(fs.readFileSync(safePath(root, target))),
      entry.sha256,
      target,
    );
    if (/\.(sysml|kerml)$/.test(target)) sourceFiles++;
  }
}
for (const { artifact, project } of projects.values())
  for (const usage of project.usage ?? []) {
    const found = projects.get(usage.resource);
    assert(found, `Missing library dependency ${usage.resource}`);
    assert.equal(
      found.project.version,
      usage.versionConstraint,
      `Unsupported dependency constraint ${usage.resource}`,
    );
    dependencies.push({
      from: artifact.id,
      to: found.artifact.id,
      required_version: usage.versionConstraint,
      actual_version: found.project.version,
      verified: true,
    });
  }
const report = {
  checked_at: new Date().toISOString(),
  lock_sha256: hash(fs.readFileSync(path.join(root, "standards/lock.json"))),
  result: "pass",
  artifacts: lock.artifacts.length,
  artifact_bytes: totalBytes,
  library_source_files: sourceFiles,
  dependency_edges: dependencies,
  artifact_discrepancies: discrepancies,
  normative_metamodel: verifyNormativeArtifacts(root),
  sysml_2_0: verifySysmlArtifacts(root),
  scope:
    "Original artifact and extracted-entry byte verification; project metadata dependency closure. Whole-library language semantics are not certified by this check.",
};
fs.writeFileSync(
  path.join(root, "verification/standards-integrity.json"),
  JSON.stringify(report, null, 2) + "\n",
);
console.log(JSON.stringify(report, null, 2));
