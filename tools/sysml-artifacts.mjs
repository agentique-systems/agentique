import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { unzipSync } from "fflate";
import { hash, safePath } from "./extract.mjs";
import { verifySystemsPublicationFreshness } from "./sysml-publication-stale.mjs";

export const sysmlLockPath = "standards/normative/sysml-2.0/lock.json";
export const librarySetPath = "standards/normative/sysml-2.0/library-set.json";

export function readSysmlLock(root) {
  const lock = JSON.parse(fs.readFileSync(safePath(root, sysmlLockPath)));
  assert.equal(lock.format, "agentique-normative-metamodel-lock/1");
  assert.equal(lock.specification, "SysML");
  assert.equal(lock.version, "2.0");
  assert.equal(lock.metamodel_uri, "https://www.omg.org/spec/SysML/20250201");
  const expected = new Map([
    ["SysML.xmi", ["primary", true, "ptc/25-02-15"]],
    ["SysML.json", ["cross-check", true, "ptc/25-04-32"]],
    ["Systems-Library.kpar", ["systems-library", true, "ptc/25-04-24"]],
  ]);
  for (const artifact of lock.artifacts) {
    assert.deepEqual(
      [artifact.role, artifact.normative, artifact.omg_file_id],
      expected.get(artifact.filename),
      artifact.filename,
    );
    expected.delete(artifact.filename);
    assert.equal(artifact.specification, "SysML");
    assert.equal(artifact.version, "2.0");
    assert.equal(artifact.source, `${lock.metamodel_uri}/${artifact.filename}`);
    assert.equal(
      artifact.path,
      artifact.filename.endsWith(".kpar")
        ? `standards/artifacts/${artifact.filename}`
        : `standards/normative/sysml-2.0/${artifact.filename}`,
    );
    validateMetadata(artifact);
  }
  assert.equal(expected.size, 0, "missing SysML input");
  assert.equal(lock.informative_example.normative, false);
  assert.equal(lock.informative_example.role, "acceptance-example");
  assert.equal(lock.informative_example.omg_file_id, "ptc/25-04-31");
  assert.equal(lock.informative_example.status, "unavailable");
  return lock;
}

function validateMetadata(artifact) {
  assert.match(artifact.sha256, /^[a-f0-9]{64}$/);
  assert(Number.isSafeInteger(artifact.bytes) && artifact.bytes > 0);
  for (const field of [
    "specification",
    "version",
    "source",
    "omg_file_id",
    "role",
    "representation",
    "media_type",
    "retrieved_at",
  ])
    assert.equal(typeof artifact[field], "string", field);
  assert(Number.isFinite(Date.parse(artifact.retrieved_at)));
  assert.equal(typeof artifact.normative, "boolean");
}

export function checkBytes(artifact, bytes) {
  assert.equal(hash(bytes), artifact.sha256, `Changed bytes: ${artifact.path}`);
  assert.equal(bytes.length, artifact.bytes, artifact.path);
}

export function librarySetIdentity(artifacts) {
  const pins = artifacts
    .map((a) => [a.source, a.sha256])
    .sort((a, b) => a[0].localeCompare(b[0], "en"));
  return `sha256:${hash(Buffer.from(JSON.stringify(["agentique-library-content-set/1", pins])))}`;
}

export function inspectKpar(bytes) {
  const entries = unzipSync(bytes);
  const projects = Object.keys(entries).filter(
    (p) => p === ".project.json" || p.endsWith("/.project.json"),
  );
  assert.equal(projects.length, 1, "KPAR must have one project root");
  const prefix = projects[0].slice(0, -".project.json".length);
  const decode = (name) =>
    JSON.parse(new TextDecoder().decode(entries[prefix + name]));
  const project = decode(".project.json"),
    meta = decode(".meta.json");
  return { prefix, project, meta, entries };
}

export function verifySysmlArtifacts(root) {
  const lock = readSysmlLock(root);
  for (const artifact of lock.artifacts)
    checkBytes(artifact, fs.readFileSync(safePath(root, artifact.path)));
  const set = JSON.parse(fs.readFileSync(safePath(root, librarySetPath)));
  assert.equal(set.format, "agentique-library-content-set/1");
  assert.equal(set.id, librarySetIdentity(set.artifacts));
  assert.equal(
    new Set(set.artifacts.map((a) => a.source)).size,
    set.artifacts.length,
  );
  const actual = new Map();
  for (const artifact of set.artifacts) {
    validateMetadata(artifact);
    assert.equal(artifact.normative, true);
    assert.equal(artifact.role, "library");
    const bytes = fs.readFileSync(safePath(root, artifact.path));
    checkBytes(artifact, bytes);
    const baseline = JSON.parse(
      fs.readFileSync(safePath(root, artifact.reused_from)),
    );
    const original = baseline.artifacts.find((a) => a.path === artifact.path);
    assert(original, artifact.path);
    assert.equal(original.sha256, artifact.sha256);
    assert.equal(original.source, artifact.source);
    assert.equal(original.document_id, artifact.omg_file_id);
    const { prefix, project, meta, entries } = inspectKpar(bytes);
    assert.deepEqual(
      { prefix, project, metamodel: meta.metamodel },
      artifact.kpar,
    );
    assert.deepEqual(
      Object.entries(entries)
        .map(([entry, data]) => ({
          path: entry,
          sha256: hash(data),
          bytes: data.length,
        }))
        .sort((a, b) => a.path.localeCompare(b.path, "en")),
      artifact.entries,
    );
    actual.set(artifact.source, project);
  }
  const visited = new Set(),
    pending = [set.root_resource],
    edges = [];
  while (pending.length) {
    const resource = pending.pop();
    if (visited.has(resource)) continue;
    visited.add(resource);
    const project = actual.get(resource);
    assert(project, `Missing library dependency ${resource}`);
    for (const usage of project.usage ?? []) {
      assert.equal(
        actual.get(usage.resource)?.version,
        usage.versionConstraint,
        usage.resource,
      );
      edges.push({
        from: resource,
        to: usage.resource,
        version: usage.versionConstraint,
      });
      pending.push(usage.resource);
    }
  }
  assert.equal(
    visited.size,
    actual.size,
    "Unrequired optional libraries included",
  );
  assert.equal(
    set.root_resource,
    lock.artifacts.find((a) => a.role === "systems-library").source,
  );
  return {
    lock: sysmlLockPath,
    artifacts: lock.artifacts,
    library_set: set.id,
    dependency_edges: edges,
    informative_example: lock.informative_example,
    accepted_publication: verifySystemsPublicationFreshness(root),
  };
}

// Explicit maintenance entry point. All remote and existing local bytes are
// checked before any missing file is created. Tests inject an offline fetcher.
export async function acquirePinned(root, artifacts, fetcher) {
  const inputs = [];
  for (const artifact of artifacts) {
    const bytes = await fetcher(artifact.source);
    checkBytes(artifact, bytes);
    const target = safePath(root, artifact.path);
    if (fs.existsSync(target)) checkBytes(artifact, fs.readFileSync(target));
    inputs.push({ target, bytes });
  }
  for (const { target, bytes } of inputs) {
    if (fs.existsSync(target)) continue;
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, bytes, { flag: "wx" });
  }
}
