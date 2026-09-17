// Run from the repository root. Capture actual exit statuses and raw tool output.
import fs from "node:fs";
import { spawnSync } from "node:child_process";

const directory = "verification/kerml-typed-views";
const commands = [
  ["format", "cargo fmt --all -- --check"],
  ["clippy", "cargo clippy --workspace --all-targets -- -D warnings"],
  ["rust-tests", "cargo test --workspace"],
  ["frontend-check", "npm run check"],
  ["frontend-build", "npm run build"],
  ["node-tests", "npm test"],
  ["browser-tests", "npm run test:e2e"],
  ["generated", "cargo run --locked --offline -p agq-metamodel-gen -- --check"],
  ["rustdoc", "cargo doc --no-deps -p agq-kerml -p agq-kernel"],
  ["dependencies", "cargo tree -p agq-kerml -e normal"],
];
fs.mkdirSync(directory, { recursive: true });
const results = [];
for (const [name, command] of commands) {
  console.log(`Running: ${command}`);
  const browserArtifacts = name === "browser-tests" ? [
    "verification/browser-results.json",
    "verification/screenshots/console-desktop.png",
    "verification/screenshots/console-mobile.png",
  ].filter((path) => fs.existsSync(path)).map((path) => [path, fs.readFileSync(path)]) : [];
  const started = new Date();
  const result = spawnSync(command, {
    shell: true,
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
    env: name === "rustdoc" ? { ...process.env, RUSTDOCFLAGS: "-D warnings" } : process.env,
  });
  fs.writeFileSync(`${directory}/${name}.txt`, `${result.stdout ?? ""}${result.stderr ?? ""}`);
  if (name === "browser-tests" && fs.existsSync("verification/browser-results.json")) {
    fs.copyFileSync("verification/browser-results.json", `${directory}/browser-results.json`);
  }
  for (const [path, contents] of browserArtifacts) fs.writeFileSync(path, contents);
  results.push({
    command, started: started.toISOString(), durationMs: Date.now() - started.getTime(),
    exitCode: result.status, signal: result.signal, error: result.error?.message ?? null,
    log: `${name}.txt`,
  });
  fs.writeFileSync(`${directory}/results.json`, `${JSON.stringify(results, null, 2)}\n`);
  console.log(`${name}: exit ${result.status}`);
}
process.exitCode = results.every((r) => r.exitCode === 0) ? 0 : 1;
