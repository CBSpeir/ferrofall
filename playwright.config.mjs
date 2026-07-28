import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/web",
  snapshotPathTemplate: "{testDir}/{testFilePath}-snapshots/{arg}{ext}",
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:8080",
    browserName: "chromium",
    colorScheme: "dark",
    viewport: { width: 960, height: 720 },
  },
  webServer: {
    command: "python3 -m http.server 8080 --directory dist",
    url: "http://127.0.0.1:8080",
    reuseExistingServer: !process.env.CI,
  },
});
