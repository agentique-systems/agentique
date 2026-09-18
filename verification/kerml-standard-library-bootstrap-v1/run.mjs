// Run from the repository root. Evidence directories are append-only.
import fs from "node:fs";
import { spawn } from "node:child_process";

const runtime = [
  ["kerml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-runtime --check"],
  ["sysml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check"],
];
const gates = {
  gate2: [
    ["corpus", "cargo run --locked --offline -p agq-standard-libraries --example syntax_audit -- --output verification/kerml-standard-library-bootstrap-v1/gate-2/syntax-quality.json"],
    ["repeatable-corpus", "cargo test --locked --offline -p agq-standard-libraries --test corpus"],
    ["grammar-inventory", "python -X utf8 tools/kerml-grammar/inventory.py"],
    ["grammar-tables", "python -X utf8 tools/kerml-grammar/generate.py"],
    ["format", "cargo fmt --all -- --check"],
    ...runtime,
  ],
  gate1: [
    ["grammar-tables", "python -X utf8 tools/kerml-grammar/generate.py"],
    ["syntax-and-authored", "cargo test --locked --offline -p agq-kerml-syntax -p agq-kerml-text"],
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --locked --offline -p agq-kerml-syntax --all-targets -- -D warnings"],
    ["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kerml-syntax"],
    ...runtime,
  ],
  gate0: [
    ["grammar-inventory", "python -X utf8 tools/kerml-grammar/inventory.py"],
    ["grammar-tests", "python -X utf8 tools/kerml-grammar/test_inventory.py"],
    ["syntax-and-sources", "cargo test --locked --offline -p agq-kerml-syntax -p agq-standard-libraries"],
    ["format", "cargo fmt --all -- --check"],
    ...runtime,
  ],
  runtime,
  authority: [
    ["inventory", "python -X utf8 verification/kerml-standard-library-bootstrap-v1/authority.py --check"],
    ...runtime,
  ],
  project: [
    ["project", "cargo test --locked --offline -p agq-kerml-text"],
    ["format", "cargo fmt --all -- --check"],
    ...runtime,
  ],
  libraries: [
    ["libraries", "cargo test --locked --offline -p agq-standard-libraries"],
    ["syntax", "cargo test --locked --offline -p agq-kerml-syntax"],
    ["corpus", "cargo run --locked --offline -p agq-standard-libraries --example audit -- --check"],
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --locked --offline -p agq-standard-libraries -p agq-kerml-text -p agq-kerml-semantics --all-targets -- -D warnings"],
    ...runtime,
  ],
  strict: [
    ["kerml-conformance", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-conformance --check"],
    ["sysml-conformance", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-conformance --check"],
  ],
  quality: [["library-quality", "cargo run --locked --offline -p agq-standard-libraries --example audit -- --require-semantic"]],
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
    ["authority", "python -X utf8 verification/kerml-standard-library-bootstrap-v1/authority.py --check"],
    ["corpus", "cargo run --locked --offline -p agq-standard-libraries --example audit -- --check"],
    ["independent-runtime", "python -X utf8 verification/language-core-completion-v3/independent_xmi.py --check"],
    ["preservation", "python -X utf8 verification/kerml-standard-library-bootstrap-v1/preservation.py"],
  ],
};
const gate = process.argv[2];
if (!Object.hasOwn(gates, gate)) throw new Error("Specify a gate: authority, project, libraries, runtime, strict, quality, final");
const label = process.argv[3] ?? gate;
if (!/^[a-z0-9-]+$/.test(label)) throw new Error("Invalid evidence label");
const directory = `verification/kerml-standard-library-bootstrap-v1/${label}`;
if (fs.existsSync(directory)) throw new Error("Evidence already exists; use a fresh label");
fs.mkdirSync(directory, { recursive: true });
const results = [];
for (const [name, command, artifacts = []] of gates[gate]) {
  console.log(`Running: ${command}`);
  const originals = artifacts.map(file => [file, fs.existsSync(file) ? fs.readFileSync(file) : null]);
  const started = new Date();
  const log = fs.openSync(`${directory}/${name}.txt`, "w");
  const result = await new Promise(resolve => {
    const child = spawn(command, { shell: true, windowsHide: true, stdio: ["ignore", log, log], env: { ...process.env, CARGO_NET_OFFLINE: "true", RUSTDOCFLAGS: "-D warnings" } });
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
