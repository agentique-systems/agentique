// Run from the repository root. Evidence directories are append-only.
import fs from "node:fs";
import { spawn } from "node:child_process";

const runtime = [
  ["kerml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-runtime --check"],
  ["sysml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check"],
];
const gates = {
  runtime,
  repair: [["format", "cargo fmt --all -- --check"], ["clippy", "cargo clippy --workspace --all-targets -- -D warnings"], ["rust-tests", "cargo test --workspace"]],
  docs: [["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-sysml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-standard-libraries -p agq-metamodel-gen"]],
  authority: [
    ["four-models", "python -X utf8 verification/kerml-library-content-errata-publication-v5/verify-four-models.py", ["verification/kerml-library-content-errata-publication-v5/four-model-witnesses.json"]],
    ["authority-stop", "python -X utf8 verification/kerml-library-content-errata-publication-v5/verify-authority-stop.py", ["verification/kerml-library-content-errata-publication-v5/authority-stop.json"]],
    ["patch-diff", "python -X utf8 verification/kerml-library-content-errata-publication-v5/verify-patch.py", ["verification/kerml-library-content-errata-publication-v5/operational-patch-diff.json"]],
    ["source-assertions", "python -X utf8 verification/kerml-library-content-errata-publication-v5/verify-source-assertions.py", ["verification/kerml-library-content-errata-publication-v5/source-assertion-diff.json"]],
    ["preservation", "python -X utf8 verification/kerml-library-content-errata-publication-v5/preservation.py"],
  ],
  product: [
    ["registers", "node tools/registers.mjs", ["verification/traceability.json", "standards/api-coverage.json"]],
    ["extraction", "node tools/extract.mjs --check"],
    ["workspace-build", "cargo build --workspace --locked"],
    ["frontend-format", "npm run format:check"],
    ["independent-starter", "node node_modules/sysml-validate/out/main.js models/ --library standards/libraries-2026-04 --no-config --format json --out {directory}/independent-validation.json"],
    ["independent-fixtures", "node node_modules/sysml-validate/out/main.js tests/fixtures/ --library standards/libraries-2026-04 --no-config --format json --out {directory}/independent-fixtures.json"],
    ["official-pilot", "node tools/pilot-validate.mjs", ["verification/pilot-validation.json", "verification/logs/pilot-validation.txt"]],
    ["process-recovery", "node tools/process-test.mjs", ["verification/process-recovery.json"]],
    ["headless-demo", `target\\debug\\agentique.exe --workspace .workspaces/verified-kerml-v8-${Date.now()}.db demo`],
  ],
  strict: [
    ["kerml-conformance", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-conformance --check"],
    ["sysml-conformance", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-conformance --check"],
  ],
  final: [
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --workspace --all-targets -- -D warnings"],
    ["rust-tests", "cargo test --workspace"],
    ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
    ...runtime,
    ["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-sysml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-standard-libraries -p agq-metamodel-gen"],
    ["standards", "npm run standards:check", ["verification/standards-integrity.json"]],
    ["frontend-check", "npm run check"],
    ["frontend-build", "npm run build"],
    ["node-tests", "npm test"],
    ["browser-tests", "npm run test:e2e", ["verification/browser-results.json", "verification/screenshots/console-desktop.png", "verification/screenshots/console-mobile.png"]],
    ["grammar-inventory", "python -X utf8 tools/kerml-grammar/inventory.py"],
    ["grammar-tables", "python -X utf8 tools/kerml-grammar/generate.py"],
    ["grammar-tests", "python -X utf8 tools/kerml-grammar/test_inventory.py"],
    ["historical-lexical-corpus", "cargo run --locked --offline -p agq-standard-libraries --example audit -- --check"],
    ["independent-runtime", "python -X utf8 verification/language-core-completion-v3/independent_xmi.py --check"],
    ["preservation", "python -X utf8 verification/kerml-library-content-errata-publication-v5/preservation.py"],
  ],
};
const gate = process.argv[2];
if (!Object.hasOwn(gates, gate)) throw new Error("Specify a gate: authority, runtime, strict, product, repair, docs, final");
const label = process.argv[3] ?? gate;
if (!/^[a-z0-9-]+$/.test(label)) throw new Error("Invalid evidence label");
const directory = `verification/kerml-semantic-closure-v8/${label}`;
if (fs.existsSync(directory)) throw new Error("Evidence already exists; use a fresh label");
fs.mkdirSync(directory, { recursive: true });
const results = [];
for (const [name, template, artifacts = []] of gates[gate]) {
  const command = template.replaceAll("{directory}", directory);
  console.log(`Running: ${command}`);
  const originals = artifacts.map(file => [file, fs.existsSync(file) ? fs.readFileSync(file) : null]);
  const started = new Date();
  const log = fs.openSync(`${directory}/${name}.txt`, "w");
  const result = await new Promise(resolve => {
    const child = spawn(command, { shell: true, windowsHide: true, stdio: ["ignore", log, log], env: { ...process.env, CARGO_NET_OFFLINE: "true", RUSTDOCFLAGS: "-D warnings", PYTHONDONTWRITEBYTECODE: "1" } });
    child.on("error", error => resolve({ exitCode: null, error: error.message }));
    child.on("close", (exitCode, signal) => resolve({ exitCode, signal }));
  });
  fs.closeSync(log);
  for (const [file, bytes] of originals) {
    if (fs.existsSync(file)) fs.copyFileSync(file, `${directory}/${file.split("/").at(-1)}`);
    if (bytes !== null) fs.writeFileSync(file, bytes);
    else if (fs.existsSync(file)) fs.unlinkSync(file);
  }
  results.push({ command, started: started.toISOString(), durationMs: Date.now() - started.getTime(), ...result, log: `${name}.txt` });
  fs.writeFileSync(`${directory}/results.json`, `${JSON.stringify(results, null, 2)}\n`);
  console.log(`${name}: exit ${result.exitCode}`);
}
process.exitCode = results.every(result => result.exitCode === 0) ? 0 : 1;
