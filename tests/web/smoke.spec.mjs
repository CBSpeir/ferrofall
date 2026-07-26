import { expect, test } from "@playwright/test";

test("loads the title screen and starts a game", async ({ page }) => {
  const browserErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") browserErrors.push(message.text());
  });
  page.on("pageerror", (error) => browserErrors.push(error.message));

  await page.goto("./");
  await expect(page).toHaveTitle("Ferrofall");

  const canvas = page.locator("#ferrofall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await canvas.focus();
  await page.keyboard.press("Enter");

  await expect(canvas).toHaveAttribute("data-screen", "playing");
  await expect(page.locator("#app_status")).toContainText("game in progress");
  expect(browserErrors).toEqual([]);
});

test("gates undersized browser viewports", async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 500 });
  await page.goto("./");

  await expect(page.locator("#ferrofall_canvas")).toHaveAttribute(
    "data-screen",
    "viewport-too-small",
    { timeout: 30_000 },
  );
  await expect(page.locator("#app_status")).toContainText("720 by 560");
});
