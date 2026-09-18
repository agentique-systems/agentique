// Run from the repository root. Evidence directories are append-only.
import fs from "node:fs";
import { spawn } from "node:child_process";

const gates = {
  focused: [
    ["authority", "python -X utf8 verification/language-core-completion-v2/authority.py --check"],
    ["kernel", "cargo test --locked --offline -p agq-kernel"],
    ["generator", "cargo test --locked --offline -p agq-metamodel-gen"],
    ["format", "cargo fmt --all -- --check"],
    ["audit", "cargo run --locked --offline -p agq-metamodel-gen -- --audit-full --check"],
    ["independent", "python -X utf8 verification/language-core-completion-v1/direct_xmi.py --check"],
  ],
  stage0: [
    ["git", "git log -1 --format=fuller"],
    ["audits", "cargo run --locked --offline -p agq-metamodel-gen -- --audit-full --check"],
    ["kerml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-runtime --check"],
    ["sysml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check"],
  ],
  final: [
    ["format", "cargo fmt --all -- --check"],
    ["clippy", "cargo clippy --workspace --all-targets -- -D warnings"],
    ["rust-tests", "cargo test --workspace"],
    ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
    ["kerml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-runtime --check"],
    ["sysml-readiness", "cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check"],
    ["rustdoc", "cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-metamodel-gen"],
    ["standards", "npm run standards:check", ["verification/standards-integrity.json"]],
    ["frontend-check", "npm run check"],
    ["frontend-build", "npm run build"],
    ["node-tests", "npm test"],
    ["browser-tests", "npm run test:e2e", ["verification/browser-results.json", "verification/screenshots/console-desktop.png", "verification/screenshots/console-mobile.png"]],
    ["independent", "python -X utf8 verification/language-core-completion-v1/direct_xmi.py --check"],
    ["authority", "python -X utf8 verification/language-core-completion-v2/authority.py --check"],
    ["preservation", "node verification/language-core-completion-v2/preservation.mjs"],
  ],
};
const gate = process.argv[2];
if (!Object.hasOwn(gates, gate)) throw new Error("Specify stage0, focused or final");
const label = process.argv[3] ?? gate;
if (!/^[a-z0-9-]+$/.test(label)) throw new Error("Invalid evidence label");
const directory = `verification/language-core-completion-v2/${label}`;
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
