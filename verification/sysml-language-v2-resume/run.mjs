// Run from the repository root; stages run only their explicit gate commands.
import fs from "node:fs";
import { spawn } from "node:child_process";

const gates = {
  foundation: [
    ["kernel", "cargo test --locked --offline -p agq-kernel"],
    ["generator", "cargo test --locked --offline -p agq-metamodel-gen"],
    ["format", "cargo fmt --all -- --check"],
  ],
  handoff: [
    ["format", "cargo fmt --all -- --check"],
    ["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kernel"],
    ["links", "node verification/sysml-language-v2-resume/check-docs.mjs"],
    ["preservation", "node verification/sysml-language-v2-resume/check-preservation.mjs"],
  ],
  "stage-3": [
    ["runtime-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check"],
    ["blocker-regressions", "cargo test --locked --offline -p agq-metamodel-gen --test sysml_closure"],
    ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
  ],
  "stage-2": [
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --locked --offline -p agq-metamodel-gen --all-targets -- -D warnings"],
    ["generator-tests", "cargo test --locked --offline -p agq-metamodel-gen"],
    ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
    ["standards", "npm run standards:check", ["verification/standards-integrity.json"]],
  ],
  "stage-1": [
    ["artifacts", "node --test tools/normative-artifacts.test.mjs tools/sysml-artifacts.test.mjs"],
    ["standards", "npm run standards:check", ["verification/standards-integrity.json"]],
    ["kerml-current", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
  ],
  "stage-0": [
    ["kernel", "cargo test -p agq-kernel"],
    ["kerml", "cargo test -p agq-kerml"],
    ["semantics", "cargo test -p agq-kerml-semantics"],
    ["text", "cargo test -p agq-kerml-text"],
    ["extraction", "node tools/extract.mjs --check"],
    ["engineering", "npm test"],
    ["links", "node verification/sysml-language-v2-resume/check-docs.mjs"],
  ],
  final: [
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --workspace --all-targets -- -D warnings"],
    ["rust-tests", "cargo test --workspace"],
    ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
    ["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-metamodel-gen"],
    ["links", "node verification/sysml-language-v2-resume/check-docs.mjs"],
    ["standards", "npm run standards:check", ["verification/standards-integrity.json"]],
    ["frontend-check", "npm run check"],
    ["frontend-build", "npm run build"],
    ["node-tests", "npm test"],
    ["browser-tests", "npm run test:e2e", ["verification/browser-results.json", "verification/screenshots/console-desktop.png", "verification/screenshots/console-mobile.png"]],
  ],
};
const gate = process.argv[2];
if (!Object.hasOwn(gates, gate)) throw new Error("Specify an implemented stage gate or final");
const directory = `verification/sysml-language-v2-resume/${gate}`;
fs.mkdirSync(directory, { recursive: true });
const results = [];
for (const [name, command, artifacts = []] of gates[gate]) {
  console.log(`Running: ${command}`);
  const originals = artifacts.map((file) => [file, fs.existsSync(file) ? fs.readFileSync(file) : null]);
  const started = new Date();
  const log = fs.openSync(`${directory}/${name}.txt`, "w");
  const result = await new Promise((resolve) => {
    const child = spawn(command, { shell: true, windowsHide: true, stdio: ["ignore", log, log], env: { ...process.env, CARGO_NET_OFFLINE: "true", RUSTDOCFLAGS: "-D warnings" } });
    child.on("error", (error) => resolve({ exitCode: null, error: error.message }));
    child.on("close", (exitCode, signal) => resolve({ exitCode, signal }));
  });
  fs.closeSync(log);
  for (const [file, bytes] of originals) {
    if (fs.existsSync(file)) fs.copyFileSync(file, `${directory}/${file.split("/").at(-1)}`);
    if (bytes !== null) fs.writeFileSync(file, bytes);
  }
  results.push({ command, started: started.toISOString(), durationMs: Date.now() - started.getTime(), ...result, log: `${name}.txt` });
  fs.writeFileSync(`${directory}/results.json`, `${JSON.stringify(results, null, 2)}\n`);
  console.log(`${name}: exit ${result.exitCode}`);
}
process.exitCode = results.every((result) => result.exitCode === 0) ? 0 : 1;
