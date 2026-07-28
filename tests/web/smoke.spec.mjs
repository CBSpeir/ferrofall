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

async function setStoredTheme(page, preference) {
  await page.addInitScript((value) => {
    window.localStorage.setItem("oxidefall.theme.v1", value);
  }, preference);
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
    return (
      state?.available &&
      state.ready &&
      state.musicReady &&
      state.musicPlaying &&
      state.contextState === "running"
    );
  });

  const beforeMute = await page.evaluate(
    () => window.oxidefallAudioDebugState().musicPosition,
  );
  await page.keyboard.press("m");
  await expect(page.locator("#app_status")).toContainText("SOUND MUTED");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("oxidefall.audio-muted.v1")))
    .toBe("true");
  await page.waitForTimeout(250);
  const whileMuted = await page.evaluate(() => window.oxidefallAudioDebugState());
  expect(whileMuted.muted).toBe(true);
  expect(whileMuted.musicPosition).toBeGreaterThan(beforeMute + 0.15);

  await page.keyboard.press("m");
  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-screen", "paused");
  const pausedPosition = await page.evaluate(
    () => window.oxidefallAudioDebugState().musicPosition,
  );
  await page.waitForTimeout(200);
  const stillPaused = await page.evaluate(() => window.oxidefallAudioDebugState());
  expect(stillPaused.musicPaused).toBe(true);
  expect(Math.abs(stillPaused.musicPosition - pausedPosition)).toBeLessThan(0.03);

  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-screen", "playing");
  await expect
    .poll(() => page.evaluate(() => window.oxidefallAudioDebugState().musicPaused))
    .toBe(false);
  expect(browserErrors).toEqual([]);
});

test("isolates missing music from sound effects and gameplay", async ({ page }) => {
  await page.route("**/audio/music_*.ogg", (route) => route.abort());
  await page.goto("./");

  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await canvas.focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => {
    const state = window.oxidefallAudioDebugState?.();
    return state?.ready && !state.musicAvailable;
  });

  await page.evaluate(() => {
    const play = window.oxidefallAudioPlay;
    window.__oxidefallMovePlays = 0;
    window.oxidefallAudioPlay = (name, ...args) => {
      if (name === "move_a" || name === "move_b") window.__oxidefallMovePlays += 1;
      return play(name, ...args);
    };
  });
  await page.keyboard.press("ArrowLeft");
  await expect
    .poll(() => page.evaluate(() => window.__oxidefallMovePlays))
    .toBeGreaterThan(0);
  await expect(canvas).toHaveAttribute("data-screen", "playing");
  await expect(page.locator("#app_status")).toContainText("Music is unavailable");
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

test("persists an explicit light preference across reload", async ({ page }) => {
  await page.goto("./");
  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await expect(canvas).toHaveAttribute("data-theme-preference", "system");
  await expect(canvas).toHaveAttribute("data-theme", "dark");

  await page.mouse.click(925, 33);
  await expect(canvas).toHaveAttribute("data-settings-open", "true");
  await page.mouse.click(784, 114);
  await expect(canvas).toHaveAttribute("data-theme-preference", "light");
  await expect(canvas).toHaveAttribute("data-theme", "light");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("oxidefall.theme.v1")))
    .toBe("light");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator("#theme_color")).toHaveAttribute("content", "#f1efe8");

  await page.reload();
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await expect(canvas).toHaveAttribute("data-theme-preference", "light");
  await expect(canvas).toHaveAttribute("data-theme", "light");
});

test("opening settings pauses and escape closes before resuming", async ({ page }) => {
  await page.goto("./");
  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await canvas.focus();
  await page.keyboard.press("Enter");
  await expect(canvas).toHaveAttribute("data-screen", "playing");

  await page.mouse.click(871, 41);
  await expect(canvas).toHaveAttribute("data-screen", "paused");
  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-screen", "paused");
  await page.keyboard.press("Escape");
  await expect(canvas).toHaveAttribute("data-screen", "playing");
});

test("settings volume sliders support pointer and keyboard input", async ({ page }) => {
  await setStoredTheme(page, "light");
  await page.goto("./");
  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });

  await page.mouse.click(925, 33);
  await expect(canvas).toHaveAttribute("data-settings-open", "true");
  await page.mouse.click(700, 233);
  await expect(page.locator("#app_status")).toContainText("EFFECTS 20%");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#app_status")).toContainText("EFFECTS 25%");
});

test("system mode follows live browser color-scheme changes", async ({ page }) => {
  await page.goto("./");
  const canvas = page.locator("#oxidefall_canvas");
  await expect(canvas).toHaveAttribute("data-screen", "title", {
    timeout: 30_000,
  });
  await expect(canvas).toHaveAttribute("data-theme-preference", "system");
  await expect(canvas).toHaveAttribute("data-theme", "dark");

  await page.emulateMedia({ colorScheme: "light" });
  await expect(canvas).toHaveAttribute("data-theme", "light");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

  await page.emulateMedia({ colorScheme: "dark" });
  await expect(canvas).toHaveAttribute("data-theme", "dark");
});

test("resolves the loading shell before WebAssembly starts", async ({ page }) => {
  await setStoredTheme(page, "light");
  await page.route("**/*.wasm", (route) => route.abort());
  await page.goto("./");

  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator("html")).toHaveAttribute(
    "data-theme-preference",
    "light",
  );
  await expect(page.locator("#theme_color")).toHaveAttribute("content", "#f1efe8");
  await expect(page.locator("#loading")).toBeVisible();
  await expect
    .poll(() =>
      page.locator("body").evaluate((body) => getComputedStyle(body).backgroundColor),
    )
    .toBe("rgb(241, 239, 232)");
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

for (const [name, viewport, mobile] of [
  ["desktop", { width: 960, height: 720 }, false],
  ["phone-portrait", { width: 360, height: 640 }, true],
]) {
  test(`matches the ${name} light title layout`, async ({ browser }) => {
    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: mobile ? 2 : 1,
      hasTouch: mobile,
      isMobile: mobile,
      colorScheme: "dark",
    });
    const page = await context.newPage();
    await setStoredTheme(page, "light");
    await page.goto("./");
    await expect(page.locator("#oxidefall_canvas")).toHaveAttribute(
      "data-screen",
      "title",
      { timeout: 30_000 },
    );
    await expect(page).toHaveScreenshot(`${name}-light-title.png`, {
      animations: "disabled",
      maxDiffPixelRatio: 0.01,
    });
    await context.close();
  });
}

for (const theme of ["dark", "light"]) {
  test(`matches ${theme} desktop gameplay chrome`, async ({ page }) => {
    await setStoredTheme(page, theme);
    await page.goto("./");
    const canvas = page.locator("#oxidefall_canvas");
    await expect(canvas).toHaveAttribute("data-screen", "title", {
      timeout: 30_000,
    });
    await canvas.focus();
    await page.keyboard.press("Enter");
    await expect(canvas).toHaveAttribute("data-screen", "playing");
    await expect(page).toHaveScreenshot(`desktop-${theme}-gameplay-chrome.png`, {
      animations: "disabled",
      clip: { x: 0, y: 0, width: 960, height: 90 },
      maxDiffPixelRatio: 0.01,
    });
  });
}

for (const theme of ["dark", "light"]) {
  test(`matches ${theme} desktop settings`, async ({ page }) => {
    await setStoredTheme(page, theme);
    await page.goto("./");
    const canvas = page.locator("#oxidefall_canvas");
    await expect(canvas).toHaveAttribute("data-screen", "title", {
      timeout: 30_000,
    });
    await page.mouse.click(925, 33);
    await expect(canvas).toHaveAttribute("data-settings-open", "true");
    await expect(page).toHaveScreenshot(`desktop-${theme}-settings.png`, {
      animations: "disabled",
      maxDiffPixelRatio: 0.01,
    });
  });
}

test.describe("light mobile gameplay chrome", () => {
  test.use({
    viewport: { width: 360, height: 640 },
    deviceScaleFactor: 2,
    hasTouch: true,
    isMobile: true,
  });

  test("matches light touch controls", async ({ page }) => {
    await setStoredTheme(page, "light");
    await page.goto("./");
    const canvas = page.locator("#oxidefall_canvas");
    await expect(canvas).toHaveAttribute("data-screen", "title", {
      timeout: 30_000,
    });
    await page.touchscreen.tap(180, 339);
    await expect(canvas).toHaveAttribute("data-screen", "playing");
    expect(
      await page.screenshot({
        animations: "disabled",
        clip: { x: 0, y: 480, width: 360, height: 160 },
      }),
    ).toMatchSnapshot("phone-light-controls.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});
