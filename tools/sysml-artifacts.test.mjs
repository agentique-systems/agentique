import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { root } from "./extract.mjs";
import { verifyNormativeArtifacts } from "./normative-artifacts.mjs";
import {
  sysmlLockPath,
  librarySetPath,
  readSysmlLock,
  verifySysmlArtifacts,
  acquirePinned,
} from "./sysml-artifacts.mjs";

function temporary(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "agq-sysml-inputs-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  for (const file of [sysmlLockPath, librarySetPath]) {
    const target = path.join(directory, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(path.join(root, file), target);
  }
  return directory;
}

test("SysML normative inputs and exact archive dependency closure verify offline", () => {
  const result = verifySysmlArtifacts(root);
  assert.equal(result.artifacts.length, 3);
  assert.equal(result.dependency_edges.length, 8);
  assert.equal(
    result.library_set,
    "sha256:6cceb50286d6edd411f327201b6016b5651744d4a30175c78e019810d482e928",
  );
  assert.equal(result.informative_example.normative, false);
  assert.equal(result.informative_example.status, "unavailable");
  // Fixed pre-SysML pins: adding a baseline must not silently move KerML.
  assert.deepEqual(
    verifyNormativeArtifacts(root).artifacts.map((a) => a.sha256),
    [
      "45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466",
      "e454fe4b7c04f3d95874b6c1a4e6ef056ea5874c71fd3d17e3180319c2f58ab2",
      "62d12217fcd26037fc917709e2a896600af574efd6412c110e2a711395a69849",
    ],
  );
});

test("wrong version, roles and normative/informative confusion fail", (t) => {
  const directory = temporary(t);
  const original = readSysmlLock(root);
  for (const mutate of [
    (l) => {
      l.version = "2.1";
    },
    (l) => {
      l.artifacts[0].normative = false;
    },
    (l) => {
      l.artifacts[1].role = "primary";
    },
    (l) => {
      l.informative_example.normative = true;
    },
    (l) => {
      l.artifacts.push({ ...l.informative_example, filename: "example.sysml" });
    },
  ]) {
    const changed = structuredClone(original);
    mutate(changed);
    fs.writeFileSync(
      path.join(directory, sysmlLockPath),
      JSON.stringify(changed),
    );
    assert.throws(() => readSysmlLock(directory));
  }
});

test("acquisition preflights all upstream bytes before creating any files", async (t) => {
  const directory = temporary(t),
    artifacts = readSysmlLock(root).artifacts;
  await assert.rejects(
    acquirePinned(directory, artifacts, async (source) => {
      const a = artifacts.find((a) => a.source === source);
      return a.role === "cross-check"
        ? Buffer.from("changed upstream")
        : fs.readFileSync(path.join(root, a.path));
    }),
    /Changed bytes/,
  );
  assert(!fs.existsSync(path.join(directory, artifacts[0].path)));
  await acquirePinned(directory, artifacts, async (source) =>
    fs.readFileSync(
      path.join(root, artifacts.find((a) => a.source === source).path),
    ),
  );
  const target = path.join(directory, artifacts[0].path);
  fs.appendFileSync(target, "changed local");
  await assert.rejects(
    acquirePinned(directory, artifacts, async (source) =>
      fs.readFileSync(
        path.join(root, artifacts.find((a) => a.source === source).path),
      ),
    ),
    /Changed bytes/,
  );
  assert(fs.readFileSync(target, "utf8").endsWith("changed local"));
  assert.throws(() => verifySysmlArtifacts(directory), /Changed bytes/);
});

test("correction release and archive content differ despite equal project versions", () => {
  const original = JSON.parse(fs.readFileSync(path.join(root, librarySetPath)));
  const corrected = JSON.parse(
    fs.readFileSync(path.join(root, "standards/lock.json")),
  );
  for (const a of original.artifacts) {
    const other = corrected.artifacts.find(
      (b) => b.canonical_resource === a.source,
    );
    assert(other, a.source);
    assert.notEqual(other.sha256, a.sha256);
  }
});
