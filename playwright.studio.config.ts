import { defineConfig } from "@playwright/test";

/** Real agq-studio processes only. No semantic endpoint interception is permitted. */
export default defineConfig({
  testDir: "tests/studio",
  workers: 1,
  fullyParallel: false,
  timeout: 1_800_000,
  expect: { timeout: 30_000 },
  reporter: [
    ["list"],
    [
      "json",
      {
        outputFile:
          "verification/generated/agentique-studio-first-light/browser-results.json",
      },
    ],
  ],
  use: {
    viewport: { width: 1600, height: 1100 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});
