import { expect, test } from "@playwright/test";

function parseTouchRegions(encoded) {
  return new Map(
    encoded.split(";").map((entry) => {
      const [name, values] = entry.split(":");
      const [x, y, width, height] = values.split(",").map(Number);
      return [name, { x, y, width, height }];
    }),
  );
}

function center(region) {
  return {
    x: region.x + region.width / 2,
    y: region.y + region.height / 2,
  };
}

async function dispatchTouches(client, type, points) {
  await client.send("Input.dispatchTouchEvent", {
    type,
    touchPoints: points.map((point, index) => ({
      ...point,
      id: index + 1,
      radiusX: 8,
      radiusY: 8,
      force: 1,
    })),
  });
}

test("loads the title screen and starts a game", async ({ page }) => {
  const browserErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") browserErrors.push(message.text());
  });
  page.on("pageerror", (error) => browserErrors.push(error.message));

  await page.goto("./");
  await expect(page).toHaveTitle("Oxidefall");

  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await canvas.focus();
  await page.keyboard.press("Enter");

  await expect(canvas).toHaveAttribute("data-screen", "playing");
  await expect(page.locator("#app_status")).toContainText("game in progress");
  await page.waitForFunction(() => {
    const state = window.oxidefallAudioDebugState?.();
    return state?.available && state.ready && state.contextState === "running";
  });

  await page.keyboard.press("m");
  await expect(page.locator("#app_status")).toContainText("SOUND MUTED");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("oxidefall.audio-muted.v1")))
    .toBe("true");
  expect(browserErrors).toEqual([]);
});

test("gates undersized browser viewports", async ({ page }) => {
  await page.setViewportSize({ width: 300, height: 480 });
  await page.goto("./");

  await expect(page.locator("#oxidefall_canvas")).toHaveAttribute(
    "data-screen",
    "viewport-too-small",
    { timeout: 30_000 },
  );
  await expect(page.locator("#app_status")).toContainText("320 by 500");
});

test("uses the compact HUD in a narrow keyboard viewport", async ({ page }) => {
  await page.setViewportSize({ width: 500, height: 500 });
  await page.goto("./");

  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await expect(canvas).toHaveAttribute("data-layout", "compact-portrait");
  await expect(canvas).toHaveAttribute("data-touch-controls", "hidden");
});

test.describe("mobile touch play", () => {
  test.use({
    viewport: { width: 360, height: 640 },
    deviceScaleFactor: 2,
    hasTouch: true,
    isMobile: true,
  });

  test("supports multi-touch controls and orientation pause", async ({
    page,
    context,
  }) => {
    const browserErrors = [];
    page.on("console", (message) => {
      if (message.type() === "error") browserErrors.push(message.text());
    });
    page.on("pageerror", (error) => browserErrors.push(error.message));

    await page.goto("./");
    const canvas = page.locator("#oxidefall_canvas");
    await expect(canvas).toHaveAttribute("data-screen", "title", {
      timeout: 30_000,
    });
    await expect(canvas).toHaveAttribute("data-layout", "compact-portrait");

    await page.touchscreen.tap(180, 339);
    await expect(canvas).toHaveAttribute("data-screen", "playing");
    await expect(canvas).toHaveAttribute("data-touch-controls", "visible");
    await page.evaluate(() => {
      const play = window.oxidefallAudioPlay;
      window.__oxidefallRotationPlays = 0;
      window.oxidefallAudioPlay = (name, ...args) => {
        if (name === "rotate") window.__oxidefallRotationPlays += 1;
        return play(name, ...args);
      };
    });
    await expect(page).toHaveScreenshot("phone-portrait-controls.png", {
      animations: "disabled",
      clip: { x: 0, y: 480, width: 360, height: 160 },
      maxDiffPixelRatio: 0.01,
    });

    const regions = parseTouchRegions(
      await canvas.getAttribute("data-touch-regions"),
    );
    expect([...regions.keys()]).toEqual([
      "hold",
      "left",
      "soft-drop",
      "right",
      "rotate-ccw",
      "rotate-cw",
      "hard-drop",
    ]);
    for (const region of regions.values()) {
      expect(region.width).toBeGreaterThanOrEqual(48);
      expect(region.height).toBeGreaterThanOrEqual(48);
    }

    const client = await context.newCDPSession(page);
    const clockwise = center(regions.get("rotate-cw"));
    const counterclockwise = center(regions.get("rotate-ccw"));
    await dispatchTouches(client, "touchStart", [clockwise]);
    await expect(canvas).toHaveAttribute("data-touch-active", "rotate-cw");
    await expect
      .poll(() => page.evaluate(() => window.__oxidefallRotationPlays))
      .toBe(1);
    await dispatchTouches(client, "touchMove", [{ x: 180, y: 400 }]);
    await expect(canvas).toHaveAttribute("data-touch-active", "rotate-cw");
    await dispatchTouches(client, "touchMove", [clockwise]);
    await expect(canvas).toHaveAttribute("data-touch-active", "rotate-cw");
    await dispatchTouches(client, "touchEnd", []);
    await expect(canvas).toHaveAttribute("data-touch-active", "");
    await expect
      .poll(() => page.evaluate(() => window.__oxidefallRotationPlays))
      .toBe(1);

    await dispatchTouches(client, "touchStart", [counterclockwise]);
    await expect(canvas).toHaveAttribute("data-touch-active", "rotate-ccw");
    await expect
      .poll(() => page.evaluate(() => window.__oxidefallRotationPlays))
      .toBe(2);
    await dispatchTouches(client, "touchEnd", []);
    await expect(canvas).toHaveAttribute("data-touch-active", "");
    await expect
      .poll(() => page.evaluate(() => window.__oxidefallRotationPlays))
      .toBe(2);

    await dispatchTouches(client, "touchStart", [
      center(regions.get("left")),
      clockwise,
    ]);
    await expect(canvas).toHaveAttribute(
      "data-touch-active",
      /left.*rotate-cw|rotate-cw.*left/,
    );
    await dispatchTouches(client, "touchEnd", []);
    await expect(canvas).toHaveAttribute("data-touch-active", "");
    await expect
      .poll(() => page.evaluate(() => window.__oxidefallRotationPlays))
      .toBe(3);

    await page.setViewportSize({ width: 640, height: 360 });
    await expect(canvas).toHaveAttribute("data-screen", "paused");
    await expect(canvas).toHaveAttribute("data-layout", "compact-landscape");
    await expect(canvas).toHaveAttribute("data-touch-controls", "hidden");

    await page.touchscreen.tap(320, 194);
    await expect(canvas).toHaveAttribute("data-screen", "playing");
    await expect(canvas).toHaveAttribute("data-touch-controls", "visible");
    await expect(page).toHaveScreenshot("phone-landscape-controls.png", {
      animations: "disabled",
      clip: { x: 400, y: 210, width: 240, height: 150 },
      maxDiffPixelRatio: 0.01,
    });
    expect(browserErrors).toEqual([]);
  });
});

for (const [name, viewport, mobile] of [
  ["desktop", { width: 960, height: 720 }, false],
  ["phone-portrait", { width: 360, height: 640 }, true],
  ["phone-landscape", { width: 640, height: 360 }, true],
  ["tablet-portrait", { width: 768, height: 1024 }, true],
]) {
  test(`matches the ${name} title layout`, async ({ browser }) => {
    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: mobile ? 2 : 1,
      hasTouch: mobile,
      isMobile: mobile,
    });
    const page = await context.newPage();
    await page.goto("./");
    await expect(page.locator("#oxidefall_canvas")).toHaveAttribute(
      "data-screen",
      "title",
      { timeout: 30_000 },
    );
    await expect(page).toHaveScreenshot(`${name}-title.png`, {
      animations: "disabled",
      maxDiffPixelRatio: 0.01,
    });
    await context.close();
  });
}
