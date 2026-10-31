import {test, expect} from '@playwright/test';
import {readFileSync} from 'node:fs';

const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');

test('browser onboarding walks from API credentials to the drive', async ({page}) => {
  let stage = 'api';
  await page.route('http://tdrive.test/**', route => {
    const url = new URL(route.request().url());
    if (url.pathname === '/') return route.fulfill({contentType: 'text/html', body: html});
    if (url.pathname === '/api/onboarding/status') {
      return route.fulfill({json: {ok: true, onboarding: {
        stage,
        configured: stage !== 'api',
        logged_in: stage === 'code' || stage === 'drive' || stage === 'ready',
        active_drive_id: stage === 'ready' ? 7 : 0,
        error: '',
      }}});
    }
    if (url.pathname === '/api/onboarding/setup') { stage = 'login'; return route.fulfill({json: {ok: true}}); }
    if (url.pathname === '/api/onboarding/login') { stage = 'code'; return route.fulfill({status: 202, json: {ok: true}}); }
    if (url.pathname === '/api/onboarding/code') { stage = 'drive'; return route.fulfill({json: {ok: true}}); }
    if (url.pathname === '/api/onboarding/drives') {
      return route.fulfill({json: {ok: true, status: 'selection_required', candidates: [
        {id: '123', title: 'My Drive', has_activity: true, recommended: true},
      ]}});
    }
    if (url.pathname === '/api/onboarding/drive') { stage = 'ready'; return route.fulfill({json: {ok: true}}); }
    if (url.pathname === '/api/status') return route.fulfill({json: {ok: true, status: {drive_id: 7, drive_title: 'My Drive'}}});
    if (url.pathname === '/api/list') return route.fulfill({json: {ok: true, path: '/', entries: []}});
    return route.abort();
  });
  await page.setViewportSize({width: 1280, height: 900});
  await page.goto('http://tdrive.test/');

  await expect(page.locator('#onboarding')).toBeVisible();
  await page.locator('#obApiId').fill('12345');
  await page.locator('#obApiHash').fill('abcdef0123456789abcdef0123456789');
  await page.getByRole('button', {name: '계속'}).click();

  await expect(page.locator('#obPhone')).toBeVisible();
  await page.locator('#obPhone').fill('+821012345678');
  await page.getByRole('button', {name: '로그인'}).click();

  await expect(page.locator('#obCode')).toBeVisible();
  await page.locator('#obCode').fill('12345');
  await page.getByRole('button', {name: '확인'}).click();

  await expect(page.getByText('My Drive')).toBeVisible();
  await page.getByRole('button', {name: '이 드라이브 사용'}).click();

  await expect(page.locator('#onboarding')).toBeHidden();
  await expect(page.locator('#navbar')).toBeVisible();
  await expect(page.locator('#files')).toBeVisible();
});
