// Sequential full-corpus audits: their proof caches must not compete for RAM.
import fs from 'node:fs';
import { spawn } from 'node:child_process';
const base = 'verification/kerml-name-resolution-errata-publication-v4';
let failed = false;
for (const [gate, label] of [['obligations', 'obligations-7'], ['published', 'published-2'], ['v1', 'v1-2'], ['comparison', 'comparison-1'], ['inventory', 'inventory-1'], ['summarize', 'summary-1']]) {
  if (gate === 'inventory') {
    while (!fs.existsSync(`${base}/quality-4/results.json`)) {
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
  }
  const code = await new Promise(resolve => {
    const child = spawn(process.execPath, [`${base}/run.mjs`, gate, label], { windowsHide: true, stdio: 'inherit' });
    child.on('close', resolve);
  });
  failed ||= code !== 0;
}
process.exitCode = failed ? 1 : 0;
