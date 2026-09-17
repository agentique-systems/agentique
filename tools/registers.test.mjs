import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { root } from "./extract.mjs";
const read = (f) => fs.readFileSync(path.join(root, f), "utf8");
test("every requirement has a declaration, implementation, test and recorded evidence reference", () => {
  const requirements = JSON.parse(read("requirements.json")),
    register = JSON.parse(read("verification/traceability.json"));
  assert.equal(register.requirements.length, 20);
  assert.deepEqual(
    register.requirements.map((r) => r.id).sort(),
    requirements.map((r) => r.id).sort(),
  );
  for (const r of register.requirements) {
    assert(fs.existsSync(path.join(root, r.implementation)), r.implementation);
    assert(r.evidence.startsWith("verification/"));
    for (const link of r.tests) {
      const [file, name] = link.split("#");
      assert(read(file).includes(name), link);
    }
  }
  const lock = JSON.parse(read("standards/lock.json")),
    coverage = JSON.parse(read("standards/coverage.json"));
  for (const c of coverage.capabilities) {
    for (const key of [
      "parsing",
      "representation",
      "validation",
      "interchange",
      "execution",
    ])
      assert(c[key], `${c.feature} ${key}`);
    for (const dep of c.dependencies)
      assert(
        lock.artifacts.some((a) => a.id === dep),
        dep,
      );
  }
});
test("dependency direction keeps modelling and simulation independent of adapters", () => {
  for (const name of [
    "model",
    "syntax",
    "semantics",
    "workspace",
    "simulation",
  ]) {
    const manifest = read(`crates/${name}/Cargo.toml`);
    for (const forbidden of [
      "reqwest",
      "rusqlite",
      "axum",
      "agq-application",
      "agq-storage",
      "agq-assistant",
    ])
      assert(!manifest.includes(forbidden), `${name} imports ${forbidden}`);
    const source = read(`crates/${name}/src/lib.rs`);
    for (const effect of ["std::process::Command", "std::net::", "reqwest::"])
      assert(!source.includes(effect), `${name}: ${effect}`);
  }
});
