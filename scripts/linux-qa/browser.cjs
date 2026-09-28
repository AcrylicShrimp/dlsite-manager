// Interact with the real Linux desktop through noVNC, not the app's DOM.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const state = path.join(root, '.linux-qa');
const { chromium } = require(path.join(state, 'browser/node_modules/playwright'));

(async () => {
  const [action = 'screen', first, second] = process.argv.slice(2);
  if (!['screen', 'click', 'key', 'type'].includes(action)) {
    throw new Error('Usage: browser [screen | click X Y | key KEY | type TEXT]');
  }
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 800 } });
    await page.goto('http://127.0.0.1:6080/vnc.html?autoconnect=true&resize=off');
    await page.locator('#noVNC_password_input').fill(
      fs.readFileSync(path.join(state, 'vnc-password'), 'utf8').trim(),
    );
    await page.locator('#noVNC_password_button').click();
    const canvas = page.locator('canvas');
    await canvas.waitFor({ state: 'visible' });
    await page.waitForTimeout(1500);
    const box = await canvas.boundingBox();
    await canvas.focus();
    if (action === 'click') {
      const x = Number(first), y = Number(second);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0 || x >= box.width || y >= box.height) {
        throw new Error('Coordinates must be inside the Linux desktop');
      }
      await page.mouse.click(box.x + x, box.y + y);
    } else if (action === 'key') {
      await page.keyboard.press(first);
    } else if (action === 'type') {
      await page.keyboard.type(first);
    }
    await page.waitForTimeout(1500);
    const output = path.join(state, 'captures', `${Date.now()}-browser-${action}.png`);
    fs.mkdirSync(path.dirname(output), { recursive: true });
    await canvas.screenshot({ path: output });
    console.log(output);
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
