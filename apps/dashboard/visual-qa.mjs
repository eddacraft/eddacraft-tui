import { chromium } from '@playwright/test';

const browser = await chromium.launch({ headless: true });
const results = [];

async function audit(name, contextOptions, screenshotOptions) {
  const context = await browser.newContext(contextOptions);
  const page = await context.newPage();
  const consoleErrors = [];

  page.on('console', (message) => {
    if (message.type() === 'error') consoleErrors.push(message.text());
  });
  page.on('pageerror', (error) => consoleErrors.push(error.message));

  await page.goto('http://127.0.0.1:5174/', { waitUntil: 'networkidle' });
  await page.getByRole('heading', { level: 1, name: 'Protection overview' }).waitFor();

  const layout = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));

  await page.screenshot(screenshotOptions);

  if (name === 'desktop') {
    await page.locator('.dashboard-topbar button[aria-label="Search dashboard"]').click();
    await page.getByRole('dialog').waitFor();
    await page.screenshot({ path: '/tmp/dash-command.png' });
    await page.locator('[data-slot="command-input"]').fill('evidence');
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Inspect sql-injection-risk' }).click();
    await page.getByText('User-controlled input is interpolated into a database query.').waitFor();
  } else {
    await page.getByRole('tab', { name: 'Warnings (12)' }).click();
    await page.getByRole('button', { name: 'Inspect hardcoded-api-key' }).click();
    await page.locator('.dashboard-mobile-header button[aria-label="Open navigation"]').click();
    await page.getByRole('dialog').waitFor();
    await page.getByRole('button', { name: 'Close' }).click();
  }

  results.push({ name, consoleErrors, layout });
  await context.close();
}

await audit(
  'desktop',
  { colorScheme: 'dark', viewport: { width: 1487, height: 1058 } },
  { path: '/tmp/dash-desktop.png' }
);
await audit(
  'mobile',
  {
    colorScheme: 'dark',
    deviceScaleFactor: 1,
    hasTouch: true,
    isMobile: true,
    viewport: { width: 853, height: 1844 },
  },
  { path: '/tmp/dash-mobile.png' }
);

await browser.close();
console.log(JSON.stringify(results, null, 2));
