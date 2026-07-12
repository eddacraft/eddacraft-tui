import assert from 'node:assert/strict';

import { chromium } from '@playwright/test';

const dashboardUrl = 'http://127.0.0.1:5174';
const browser = await chromium.launch({ headless: true });
const results = [];

async function audit(name, contextOptions, screenshotPath) {
  const context = await browser.newContext(contextOptions);
  const page = await context.newPage();
  const consoleErrors = [];

  page.on('console', (message) => {
    if (message.type() === 'error') consoleErrors.push(message.text());
  });
  page.on('pageerror', (error) => consoleErrors.push(error.message));

  await page.goto(`${dashboardUrl}/`, { waitUntil: 'networkidle' });
  await page.getByRole('heading', { level: 1, name: 'Protection overview' }).waitFor();
  assert.equal(await page.title(), 'Anvil Dashboard');

  const homeLayout = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  assert.equal(homeLayout.scrollWidth, homeLayout.clientWidth, `${name} home page overflows`);
  assert.equal(consoleErrors.length, 0, `${name} console errors: ${consoleErrors.join(' | ')}`);
  await page.screenshot({ fullPage: false, path: screenshotPath });

  if (name === 'desktop') {
    await page.locator('.dashboard-topbar button[aria-label="Search dashboard"]').click();
    const dialog = page.getByRole('dialog');
    await dialog.waitFor();
    await dialog.locator('[data-slot="command-input"]').fill('plan');
    await assert.doesNotReject(() =>
      dialog.getByText('Plan Driver', { exact: true }).first().waitFor()
    );
    assert.equal(await dialog.getByText('Selected plan detail', { exact: true }).count(), 0);
    await page.keyboard.press('Escape');
    await page.goto(`${dashboardUrl}/plans`, { waitUntil: 'networkidle' });
    await page.getByRole('heading', { level: 1, name: 'Plan Driver' }).waitFor();

    const planLinks = page.locator('.plan-list tbody a');
    if ((await planLinks.count()) > 0) {
      await planLinks.first().click();
      const approval = page.getByRole('button', { name: 'Request approval' });
      await approval.waitFor();
      assert.equal(await approval.isDisabled(), true);
    } else {
      await page.getByText('No indexed plans', { exact: true }).waitFor();
    }
  } else {
    await page.getByRole('tab', { name: /Warnings/ }).click();
    await page.locator('.dashboard-mobile-header button[aria-label="Open navigation"]').click();
    const drawer = page.getByRole('dialog');
    await drawer.waitFor();
    await drawer.getByRole('link', { name: 'Plans' }).click();
    await page.waitForURL(`${dashboardUrl}/plans?severity=all&view=runs`);
    assert.equal(await drawer.count(), 0, 'mobile navigation drawer remained open');
    await page.locator('.plan-list, .query-error').waitFor();
  }

  const finalLayout = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  assert.equal(finalLayout.scrollWidth, finalLayout.clientWidth, `${name} final page overflows`);
  assert.equal(consoleErrors.length, 0, `${name} console errors: ${consoleErrors.join(' | ')}`);
  await page.screenshot({
    fullPage: false,
    path: screenshotPath.replace('.png', '-navigation.png'),
  });

  results.push({ name, consoleErrors, homeLayout, finalLayout, url: page.url() });
  await context.close();
}

try {
  await audit(
    'desktop',
    { colorScheme: 'dark', viewport: { width: 1487, height: 1058 } },
    '/tmp/dash-wave1-desktop.png'
  );
  await audit(
    'mobile',
    {
      colorScheme: 'dark',
      deviceScaleFactor: 1,
      hasTouch: true,
      isMobile: true,
      viewport: { width: 390, height: 844 },
    },
    '/tmp/dash-wave1-mobile-390.png'
  );
} finally {
  await browser.close();
}

console.log(JSON.stringify(results, null, 2));
