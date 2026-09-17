// Explicit acquisition only. Never imported by a build, generator, or test.
// Reproduce the reviewed lock; changing the baseline requires a new reviewed lock.
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { root, hash, safePath } from "./extract.mjs";
import { readNormativeLock } from "./normative-artifacts.mjs";

const { lock } = readNormativeLock(root);
const inputs = [];
for (const artifact of lock.artifacts) {
  const target = safePath(root, artifact.path);
  const response = await fetch(artifact.source);
  assert(response.ok, `${artifact.source}: HTTP ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(
    hash(bytes),
    artifact.sha256,
    `Upstream bytes changed: ${artifact.source}; pinned input will not be overwritten`,
  );
  assert.equal(bytes.length, artifact.bytes, artifact.path);
  if (fs.existsSync(target))
    assert.equal(hash(fs.readFileSync(target)), artifact.sha256, artifact.path);
  inputs.push({ target, bytes });
}
// Preflight all hashes before writing; exclusive creation forbids replacement.
for (const { target, bytes } of inputs) {
  if (fs.existsSync(target)) continue;
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, bytes, { flag: "wx" });
}
console.log(
  "KerML 1.0 pinned artifacts reproduced/verified without replacement.",
);
