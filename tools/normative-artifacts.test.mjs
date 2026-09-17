import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { root } from "./extract.mjs";
import {
  normativeLockPath,
  verifyNormativeArtifacts,
} from "./normative-artifacts.mjs";

test("normative metamodel hashes and roles are checked locally", () => {
  const result = verifyNormativeArtifacts(root);
  assert.equal(result.artifacts.length, 3);
  assert.deepEqual(
    result.artifacts.map((a) => a.role),
    ["primary", "cross-check", "primary-dependency"],
  );
});

test("changed normative bytes fail without modifying the original", () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "agq-normative-"));
  try {
    const relative = path.dirname(normativeLockPath);
    fs.cpSync(path.join(root, relative), path.join(temporary, relative), {
      recursive: true,
    });
    const target = path.join(temporary, relative, "KerML.xmi");
    fs.appendFileSync(target, "\nchanged\n");
    assert.throws(() => verifyNormativeArtifacts(temporary), /KerML\.xmi/);
    assert(fs.readFileSync(target, "utf8").endsWith("changed\n"));
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
});
