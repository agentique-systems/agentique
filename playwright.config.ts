import { defineConfig } from "@playwright/test";
import { randomUUID } from "node:crypto";
const workspace = `.workspaces/e2e-${randomUUID()}.db`;
export default defineConfig({
  testDir: "tests/browser",
  workers: 1,
  fullyParallel: false,
  timeout: 60000,
  reporter: [
    ["list"],
    ["json", { outputFile: "verification/browser-results.json" }],
  ],
  use: {
    baseURL: "http://127.0.0.1:7342",
    viewport: { width: 1512, height: 1100 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: `cargo run -p agq-server -- --port 7342 --workspace ${workspace}`,
    url: "http://127.0.0.1:7342",
    timeout: 120000,
    reuseExistingServer: false,
    env: {
      AGENTIQUE_SESSION_TOKEN: "agentique-e2e-local-session-only",
      AGENTIQUE_ASSISTANT: "test",
    },
  },
});
