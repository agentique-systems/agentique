// Run from the repository root. Logs and exit codes are actual command results.
import fs from "node:fs";
import { spawn } from "node:child_process";

const directory = "verification/kerml-query-foundation";
fs.mkdirSync(directory, { recursive: true });
const rustOnly = process.argv.includes("--rust");
const results = rustOnly && fs.existsSync(`${directory}/results.json`)
  ? JSON.parse(fs.readFileSync(`${directory}/results.json`, "utf8"))
      .filter((r) => !["format.txt", "clippy.txt", "rust-tests.txt"].includes(r.log))
  : [];
async function run(name, command, artifacts = []) {
  console.log(`Running: ${command}`);
  const originals = artifacts.map((path) => [path, fs.existsSync(path) ? fs.readFileSync(path) : null]);
  const started = new Date();
  let output = "";
  const result = await new Promise((resolve) => {
    const process = spawn(command, { shell: true, windowsHide: true });
    process.stdout.on("data", (data) => { output += data; });
    process.stderr.on("data", (data) => { output += data; });
    process.on("error", (error) => resolve({ exitCode: null, error: error.message }));
    process.on("close", (exitCode, signal) => resolve({ exitCode, signal }));
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
    if (!rustOnly) await run("generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check");
  })(),
  (async () => {
    if (rustOnly) return;
    await run("frontend-check", "npm run check");
    await run("frontend-build", "npm run build");
    await run("node-tests", "npm test");
    await run("standards", "npm run standards:check", ["verification/standards-integrity.json"]);
  })(),
]);
if (!rustOnly) await run("browser-tests", "npm run test:e2e", [
  "verification/browser-results.json",
  "verification/screenshots/console-desktop.png",
  "verification/screenshots/console-mobile.png",
]);
process.exitCode = results.every((r) => r.exitCode === 0) ? 0 : 1;
