import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { hash, safePath } from "./extract.mjs";

export const normativeLockPath = "standards/normative/kerml-1.0/lock.json";

export function readNormativeLock(root) {
  const bytes = fs.readFileSync(safePath(root, normativeLockPath));
  const lock = JSON.parse(bytes);
  assert.equal(lock.format, "agentique-normative-metamodel-lock/1");
  assert.equal(lock.specification, "KerML");
  assert.equal(lock.version, "1.0");
  assert.equal(lock.artifacts.length, 3);
  const expected = new Map([
    ["KerML.xmi", ["primary", "KerML", "1.0"]],
    ["KerML.json", ["cross-check", "KerML", "1.0"]],
    ["PrimitiveTypes.xmi", ["primary-dependency", "UML", "2.5.1"]],
  ]);
  for (const artifact of lock.artifacts) {
    assert.deepEqual(
      [artifact.role, artifact.specification, artifact.version],
      expected.get(artifact.filename),
      `Unexpected normative artifact ${artifact.filename}`,
    );
    expected.delete(artifact.filename);
    assert.equal(
      artifact.path,
      `${path.posix.dirname(normativeLockPath)}/${artifact.filename}`,
    );
    assert.match(artifact.sha256, /^[a-f0-9]{64}$/);
    assert(Number.isSafeInteger(artifact.bytes) && artifact.bytes > 0);
    assert.equal(new URL(artifact.source).origin, "https://www.omg.org");
    for (const field of [
      "omg_file_id",
      "retrieved_at",
      "representation",
      "media_type",
    ])
      assert.equal(typeof artifact[field], "string", field);
  }
  return { lock, lockSha256: hash(bytes) };
}

export function verifyNormativeArtifacts(root) {
  const { lock, lockSha256 } = readNormativeLock(root);
  const artifacts = lock.artifacts.map((artifact) => {
    const bytes = fs.readFileSync(safePath(root, artifact.path));
    assert.equal(hash(bytes), artifact.sha256, artifact.path);
    assert.equal(bytes.length, artifact.bytes, artifact.path);
    return {
      path: artifact.path,
      sha256: artifact.sha256,
      bytes: bytes.length,
      source: artifact.source,
      role: artifact.role,
    };
  });
  return { lock: normativeLockPath, lock_sha256: lockSha256, artifacts };
}
