import { defineConfig } from "@playwright/test";

/** Frontend contract fixtures only; real durable acceptance uses supplied accepted caches. */
export default defineConfig({
  testDir: "tests/browser",
  testMatch: "studio.spec.ts",
  workers: 1,
  timeout: 30000,
  reporter: [
    ["list"],
    [
      "json",
      { outputFile: "verification/generated/studio-browser-results.json" },
    ],
  ],
  use: {
    baseURL: "http://127.0.0.1:5174",
    viewport: { width: 1512, height: 1000 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run dev -- --port 5174",
    url: "http://127.0.0.1:5174",
    timeout: 30000,
    reuseExistingServer: false,
  },
});
