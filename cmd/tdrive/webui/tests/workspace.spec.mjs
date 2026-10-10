import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
const entries = [
  {type: 'folder', name: 'Documents', path: '/Documents'},
  {type: 'file', name: 'notes.txt', path: '/notes.txt', size: 1024, upload_time: 1700000000},
  {type: 'file', name: 'photo.jpg', path: '/photo.jpg', size: 2048, upload_time: 1700000001},
];
for (const width of [1280, 390]) {
  test(`workspace navigation and layout at ${width}px`, async ({page}) => {
    await page.setViewportSize({width, height: 800});
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.route('http://tdrive.test/**', route => {
      const url = new URL(route.request().url());
      if (url.pathname === '/') return route.fulfill({contentType: 'text/html', body: html});
      if (url.pathname === '/api/status') return route.fulfill({json: {ok: true, status: {drive_id: 7, drive_title: 'TDrive', current_path: '/'}}});
      if (url.pathname === '/api/list') return route.fulfill({json: {ok: true, path: url.searchParams.get('path') || '/', entries}});
      return route.fulfill({json: {ok: true}});
    });
    await page.goto('http://tdrive.test/');
    await expect(page.getByRole('heading', {name: '내 드라이브'})).toBeVisible();
    await expect(page.locator('#files .row')).toHaveCount(3);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.locator('#btnView').click();
    await page.locator('[data-view="icons"]').click();
    await expect(page.locator('#files .tile')).toHaveCount(3);
    await page.locator('#files .tile').filter({hasText: 'Documents'}).dblclick();
    await expect(page).toHaveURL(/path=%2FDocuments/);
    await page.locator('#btnBack').click();
    await expect(page).toHaveURL(/path=%2F(?:&|$)/);
    await page.screenshot({path: `/tmp/tdrive-web-${width}.png`, fullPage: true});
    expect(errors).toEqual([]);
  });
}
