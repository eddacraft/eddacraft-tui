import assert from 'node:assert/strict';

import { chromium } from '@playwright/test';

const dashboardUrl = 'http://127.0.0.1:5174';
const browser = await chromium.launch({ headless: true });
const results = [];

async function audit(name, contextOptions, screenshotPath) {
  const context = await browser.newContext(contextOptions);
  const page = await context.newPage();
  const consoleErrors = [];
  const externalRequests = [];

  page.on('console', (message) => {
    if (message.type() === 'error') consoleErrors.push(message.text());
  });
  page.on('pageerror', (error) => consoleErrors.push(error.message));
  page.on('request', (request) => {
    const hostname = new URL(request.url()).hostname;
    if (hostname !== '127.0.0.1') externalRequests.push(request.url());
  });

  await page.goto(`${dashboardUrl}/`, { waitUntil: 'networkidle' });
  await page.getByRole('heading', { level: 1, name: 'Protection overview' }).waitFor();
  assert.equal(await page.title(), 'anvil // dashboard');

  const homeLayout = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  assert.equal(homeLayout.scrollWidth, homeLayout.clientWidth, `${name} home page overflows`);
  assert.equal(
    await page.locator('.current-health-cards > .metric-card').count(),
    5,
    `${name} health card count`
  );
  assert.equal(consoleErrors.length, 0, `${name} console errors: ${consoleErrors.join(' | ')}`);
  assert.equal(
    externalRequests.length,
    0,
    `${name} made external requests: ${externalRequests.join(' | ')}`
  );

  const brandSnapshot = await page.evaluate(() => {
    const root = getComputedStyle(document.documentElement);
    const activeNavigation = document.querySelector(
      '.dashboard-nav a[aria-current="page"], .mobile-bottom-nav a[aria-current="page"]'
    );
    const failStatus = document.createElement('span');
    failStatus.dataset.status = 'fail';
    document.body.append(failStatus);
    const highSeverity = document.createElement('span');
    highSeverity.className = 'severity-badge-high';
    document.body.append(highSeverity);
    const panels = [...document.querySelectorAll('.panel, .protection-summary, .metric-card')].map(
      (panel) => {
        const style = getComputedStyle(panel);
        return {
          backgroundImage: style.backgroundImage,
          borderRadius: style.borderRadius,
          boxShadow: style.boxShadow,
        };
      }
    );
    const snapshot = {
      activeNavigationText: activeNavigation ? getComputedStyle(activeNavigation).color : null,
      background: getComputedStyle(document.body).backgroundColor,
      brandmarks: document.querySelectorAll('img[src="/anvil-brandmark-ember.svg"]').length,
      failStatusText: getComputedStyle(failStatus).color,
      fontFamily: root.fontFamily,
      highSeverityText: getComputedStyle(highSeverity).color,
      panels,
      tokens: {
        anvil: root.getPropertyValue('--anvil').trim(),
        brickRed: root.getPropertyValue('--brick-red').trim(),
        dullAmber: root.getPropertyValue('--dull-amber').trim(),
        edda: root.getPropertyValue('--edda').trim(),
        ghostGrey: root.getPropertyValue('--ghost-grey').trim(),
        offWhite: root.getPropertyValue('--off-white').trim(),
        structure: root.getPropertyValue('--structure').trim(),
        surface: root.getPropertyValue('--surface').trim(),
        void: root.getPropertyValue('--void').trim(),
      },
    };
    failStatus.remove();
    highSeverity.remove();
    return snapshot;
  });
  assert.deepEqual(brandSnapshot.tokens, {
    anvil: '#cc5500',
    brickRed: '#c94a4a',
    dullAmber: '#d08c38',
    edda: '#2e8b57',
    ghostGrey: '#85858a',
    offWhite: '#ebebeb',
    structure: '#2a2a2e',
    surface: '#141416',
    void: '#0d0d0f',
  });
  assert.equal(brandSnapshot.background, 'rgb(13, 13, 15)');
  assert.equal(brandSnapshot.brandmarks, 2);
  assert.equal(brandSnapshot.activeNavigationText, 'rgb(235, 235, 235)');
  assert.equal(brandSnapshot.failStatusText, 'rgb(235, 235, 235)');
  assert.equal(brandSnapshot.highSeverityText, 'rgb(235, 235, 235)');
  assert.match(brandSnapshot.fontFamily, /JetBrains Mono/);
  assert.ok(
    brandSnapshot.panels.every(
      (panel) =>
        panel.backgroundImage === 'none' &&
        panel.borderRadius === '0px' &&
        panel.boxShadow === 'none'
    ),
    `${name} panels violate the Nordic Terminal surface contract`
  );
  await page.screenshot({ fullPage: false, path: screenshotPath });

  if (name === 'mobile') {
    const affectedFilesPanel = page.locator('.affected-files-panel');
    const bottomNavigation = page.locator('.mobile-bottom-nav');
    await affectedFilesPanel.scrollIntoViewIfNeeded();
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    const [affectedFilesBox, bottomNavigationBox] = await Promise.all([
      affectedFilesPanel.boundingBox(),
      bottomNavigation.boundingBox(),
    ]);
    assert.ok(
      affectedFilesBox,
      'affected files panel is not visible at the bottom of the mobile page'
    );
    assert.ok(bottomNavigationBox, 'mobile bottom navigation is not visible');
    assert.ok(
      affectedFilesBox.y + affectedFilesBox.height <= bottomNavigationBox.y,
      'mobile bottom navigation obscures the affected files panel'
    );
    await page.screenshot({ fullPage: false, path: '/tmp/dash-wave1-mobile-390-bottom.png' });
    await page.evaluate(() => window.scrollTo(0, 0));
  }

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

  results.push({ name, brandSnapshot, consoleErrors, homeLayout, finalLayout, url: page.url() });
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
