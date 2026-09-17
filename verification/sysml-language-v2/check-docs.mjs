// Local links only: documentation verification never needs the network.
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";

const files = ["README.md", "docs/architecture.md", "docs/semantic-kernel.md",
  ...fs.readdirSync("docs/adr").filter((name) => /^000[1-6]-/.test(name)).map((name) => `docs/adr/${name}`)];
let checked = 0;
for (const file of files) {
  for (const match of fs.readFileSync(file, "utf8").matchAll(/\]\(([^)]+)\)/g)) {
    const target = match[1].split("#")[0];
    if (!target || /^[a-z]+:/i.test(target)) continue;
    assert(fs.existsSync(path.resolve(path.dirname(file), target)), `${file}: missing ${target}`);
    checked++;
  }
}
const register = JSON.parse(fs.readFileSync("standards/v2-coverage.json", "utf8"));
assert.equal(register.generation, 2);
for (const capability of register.capabilities) {
  assert(Object.hasOwn(register.status_meaning, capability.status), capability.id);
  for (const evidence of capability.evidence) assert(fs.existsSync(evidence), evidence);
}
console.log(`${checked} local documentation links and ${register.capabilities.length} generation-2 capability entries checked.`);
