// Run from the repository root. Every result below is a real process exit code.
import fs from "node:fs";
import { spawn } from "node:child_process";

const directory = "verification/kerml-text-v2";
fs.mkdirSync(directory, { recursive: true });
const results = [];
async function run(name, command, artifacts = []) {
  console.log(`Running: ${command}`);
  const originals = artifacts.map((path) => [path, fs.existsSync(path) ? fs.readFileSync(path) : null]);
  const started = new Date();
  let output = "";
  const result = await new Promise((resolve) => {
    const child = spawn(command, { shell: true, windowsHide: true });
    child.stdout.on("data", (data) => { output += data; });
    child.stderr.on("data", (data) => { output += data; });
    child.on("error", (error) => resolve({ exitCode: null, error: error.message }));
    child.on("close", (exitCode, signal) => resolve({ exitCode, signal }));
  });
  fs.writeFileSync(`${directory}/${name}.txt`, output);
  for (const [path, bytes] of originals) {
    if (fs.existsSync(path)) fs.copyFileSync(path, `${directory}/${path.split("/").at(-1)}`);
    if (bytes !== null) fs.writeFileSync(path, bytes);
  }
  results.push({ command, started: started.toISOString(), durationMs: Date.now() - started.getTime(), ...result, log: `${name}.txt` });
  fs.writeFileSync(`${directory}/results.json`, `${JSON.stringify(results, null, 2)}\n`);
  console.log(`${name}: exit ${result.exitCode}`);
}
await Promise.all([
  (async () => {
    await run("format", "cargo fmt --all -- --check");
    await run("clippy", "cargo clippy --workspace --all-targets -- -D warnings");
    await run("rust-tests", "cargo test --workspace");
    await run("generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check");
    await run("dependencies", "cargo tree -p agq-kerml-text -p agq-kerml-semantics -p agq-kernel --edges normal");
  })(),
  (async () => {
    await run("frontend-check", "npm run check");
    await run("frontend-build", "npm run build");
    await run("node-tests", "npm test");
    await run("standards", "npm run standards:check", ["verification/standards-integrity.json"]);
  })(),
]);
await run("browser-tests", "npm run test:e2e", [
  "verification/browser-results.json",
  "verification/screenshots/console-desktop.png",
  "verification/screenshots/console-mobile.png",
]);
process.exitCode = results.every((r) => r.exitCode === 0) ? 0 : 1;
