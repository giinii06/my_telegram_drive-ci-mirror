import {test, expect} from '@playwright/test';
import {readFileSync, mkdirSync} from 'node:fs';

const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
const now = Math.floor(Date.now() / 1000);
const entries = [
  {type: 'folder', name: 'Documents', path: '/Documents'},
  {type: 'file', name: 'Project brief.pdf', path: '/Project brief.pdf', size: 1258291, upload_time: now},
  {type: 'file', name: 'Budget.xlsx', path: '/Budget.xlsx', size: 24576, upload_time: now - 86400 * 2},
  {type: 'file', name: 'notes.txt', path: '/notes.txt', size: 1024, upload_time: now - 86400 * 40},
  {type: 'file', name: 'photo.jpg', path: '/photo.jpg', size: 2048000, upload_time: now},
  {type: 'file', name: 'landscape.png', path: '/landscape.png', size: 1048576, upload_time: now - 86400 * 10},
  {type: 'file', name: 'unknown.jpg', path: '/unknown.jpg', size: 2048},
  {type: 'file', name: 'Film.mp4', path: '/Film.mp4', size: 104857600, upload_time: now - 86400},
];
const shots = '/tmp/tdrive-drive-ui';
mkdirSync(shots, {recursive: true});

test.beforeEach(async ({page}) => {
  await page.route('http://tdrive.test/**', route => {
    const url = new URL(route.request().url());
    if (url.pathname === '/') return route.fulfill({contentType: 'text/html', body: html});
    if (url.pathname === '/api/list') {
      const path = url.searchParams.get('path') || '/';
      return route.fulfill({json: {ok: true, path, entries: path === '/' ? entries : []}});
    }
    if (url.pathname === '/api/find') return route.fulfill({json: {ok: true, results: entries.filter(e => e.name.includes(url.searchParams.get('q')))}});
    if (url.pathname === '/api/status') return route.fulfill({json: {ok: true, status: {drive_id: 7, drive_title: 'TDrive'}}});
    if (url.pathname === '/api/upload') return route.fulfill({json: {ok: true, job: {id: 'test-upload'}}});
    if (url.pathname === '/api/upload/status') return route.fulfill({json: {ok: true, job: {files: [{stage: 'done', progress: 100}]}}});
    if (url.pathname === '/api/mkdir') return route.fulfill({json: {ok: true}});
    return route.abort();
  });
  await page.setViewportSize({width: 1440, height: 900});
  await page.goto('http://tdrive.test/');
  await expect(page.locator('#files .row')).toHaveCount(entries.length);
});

test('filters restrict selection and preview siblings without changing the directory', async ({page}) => {
  await page.getByLabel('파일 유형', {exact: true}).selectOption('image');
  await expect(page.locator('#files .row')).toHaveCount(3);
  await page.getByLabel('모두 선택', {exact: true}).check();
  await expect(page.locator('#selection-count')).toHaveText('3개 선택됨');
  expect(await page.evaluate(() => window.state.entries.length)).toBe(entries.length);
  expect(await page.evaluate(() => window.state.sel)).toEqual(['/landscape.png', '/photo.jpg', '/unknown.jpg']);
  await page.getByLabel('선택 해제', {exact: true}).click();
  await page.getByLabel('업로드 날짜', {exact: true}).selectOption('7');
  await expect(page.locator('#files .row')).toHaveCount(1);
  await expect(page.locator('#ribbon')).toBeHidden();
  await page.locator('#files .row').dblclick();
  await expect(page.locator('#pvcap')).toHaveText('photo.jpg');
  await expect(page.locator('#pvNext')).toBeDisabled();
  await page.locator('#closePv').click();
  await page.getByLabel('선택 해제', {exact: true}).click();
  await page.getByRole('button', {name: '필터 지우기'}).click();
  await expect(page.locator('#files .row')).toHaveCount(entries.length);
  await expect(page.getByLabel('모두 선택', {exact: true})).not.toBeChecked();
  await page.locator('#files').click({position: {x: 10, y: 450}});
  await page.keyboard.press('ControlOrMeta+a');
  await expect(page.locator('#selection-count')).toHaveText('8개 선택됨');
});

test('new menu supports keyboard navigation, folder creation and upload', async ({page}) => {
  const create = page.locator('#btnCreate');
  await create.focus();
  await page.keyboard.press('ArrowDown');
  await expect(create).toHaveAttribute('aria-expanded', 'true');
  await expect(page.locator('#btnNew')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(create).toBeFocused();
  await expect(page.locator('#menuNew')).toBeHidden();
  await create.click();
  await page.locator('#btnNew').click();
  await expect(page.locator('#modal h3')).toHaveText('새 폴더');
  await page.locator('#modal input').fill('새 문서');
  const mkdir = page.waitForRequest('**/api/mkdir');
  await page.locator('#modal .primary').click();
  expect((await mkdir).postDataJSON()).toEqual({path: '/새 문서'});
  await create.click();
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#btnUpload').click();
  const upload = page.waitForRequest('**/api/upload?**');
  await (await chooser).setFiles({name: 'upload.txt', mimeType: 'text/plain', buffer: Buffer.from('TDrive UI check')});
  expect(new URL((await upload).url()).searchParams.get('path')).toBe('/');
  await expect(page.locator('#ulist')).toContainText('완료');
  await create.click();
  const directory = page.waitForEvent('filechooser');
  await page.locator('#btnUploadDir').click();
  expect(await (await directory).element().getAttribute('webkitdirectory')).not.toBeNull();
  await expect(page.locator('#menuNew')).toBeHidden();
});

test('search clears back to the directory and selection actions preserve paste', async ({page}) => {
  await page.getByRole('searchbox').fill('photo');
  await expect(page.locator('#files .row')).toHaveCount(1);
  await expect(page.getByRole('heading', {name: '검색 결과'})).toBeVisible();
  await page.getByRole('searchbox').fill('');
  await expect(page.locator('#files .row')).toHaveCount(entries.length);
  await page.locator('#files .row').filter({hasText: 'notes.txt'}).click();
  await expect(page.getByLabel('이름 바꾸기', {exact: true})).toBeEnabled();
  await page.getByLabel('복사', {exact: true}).click();
  await page.getByLabel('선택 해제', {exact: true}).click();
  await expect(page.getByLabel('파일 유형', {exact: true})).toBeVisible();
  await expect(page.getByLabel('붙여넣기', {exact: true})).toBeEnabled();
  await expect(page.locator('#ribbon')).toBeHidden();
});

test('desktop list, selection and grid surfaces', async ({page}) => {
  await page.screenshot({path: `${shots}/desktop-list.png`});
  await page.locator('#files .row').filter({hasText: 'Project brief.pdf'}).click();
  await page.getByLabel('세부 정보 창', {exact: true}).click();
  await expect(page.locator('#details')).toContainText('Project brief.pdf');
  await page.screenshot({path: `${shots}/desktop-selection.png`});
  await page.getByLabel('세부 정보 닫기').click();
  await page.getByLabel('선택 해제', {exact: true}).click();
  await page.getByLabel('그리드 보기', {exact: true}).click();
  await expect(page.locator('#files .tile')).toHaveCount(entries.length);
  await expect(page.getByLabel('그리드 보기', {exact: true})).toHaveAttribute('aria-pressed', 'true');
  await page.screenshot({path: `${shots}/desktop-grid.png`});
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

for (const width of [390, 700, 1024]) {
  test(`responsive controls and menus at ${width}px`, async ({page}) => {
    await page.setViewportSize({width, height: 844});
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (width <= 700) {
      await page.getByLabel('탐색 메뉴', {exact: true}).click();
      await expect(page.locator('#side')).toBeVisible();
      await page.locator('#btnCreate').click();
      await expect(page.locator('#menuNew')).toBeVisible();
      await page.keyboard.press('Escape');
      await page.keyboard.press('Escape');
      await expect(page.locator('#side')).toBeHidden();
    }
    await page.getByLabel('파일 유형', {exact: true}).selectOption('folder');
    await page.getByLabel('그리드 보기', {exact: true}).click();
    await page.locator('#files .tile').dblclick();
    await expect(page).toHaveURL(/path=%2FDocuments/);
    await expect(page.locator('.empty-state')).toBeVisible();
    await page.screenshot({path: `${shots}/empty-${width}.png`});
    await page.getByLabel('뒤로', {exact: true}).click();
    await expect(page.locator('#files .tile')).toHaveCount(1);
    await page.getByRole('button', {name: '필터 지우기'}).click();
    await expect(page.locator('#files .tile')).toHaveCount(entries.length);
    await page.screenshot({path: `${shots}/grid-${width}.png`});
  });
}


test('context selection refreshes actions and keyboard sorting retains focus', async ({page}) => {
  await page.locator('#files .row').filter({hasText: 'notes.txt'}).click({button: 'right'});
  await expect(page.locator('#ribbon')).toBeVisible();
  await expect(page.locator('#selection-count')).toHaveText('1개 선택됨');
  await expect(page.getByLabel('이름 바꾸기', {exact: true})).toBeEnabled();
  await page.keyboard.press('Escape');
  await page.getByLabel('세부 정보 창', {exact: true}).click();
  await page.locator('#files .row').filter({hasText: 'photo.jpg'}).click({button: 'right'});
  await expect(page.locator('#details')).toContainText('photo.jpg');
  await page.keyboard.press('Escape');
  await page.locator('#files .row').filter({hasText: 'notes.txt'}).click();
  const nameColumn = page.locator('#colhead [data-column=name]');
  await nameColumn.focus();
  await page.keyboard.press('Enter');
  await expect(nameColumn).toBeFocused();
  await expect(page.locator('#preview')).toBeHidden();
  await page.setViewportSize({width: 390, height: 844});
  await expect(page.getByRole('button', {name: '정렬', exact: true})).toBeVisible();
});


test('mobile clipboard and active filters keep header selection reachable', async ({page}) => {
  await page.setViewportSize({width: 390, height: 844});
  await page.getByLabel('파일 유형', {exact: true}).selectOption('image');
  await page.getByLabel('업로드 날짜', {exact: true}).selectOption('7');
  await page.locator('#files .row').click();
  await page.getByLabel('복사', {exact: true}).click();
  await page.getByLabel('선택 해제', {exact: true}).click();
  await expect(page.getByLabel('붙여넣기', {exact: true})).toBeEnabled();
  await page.getByLabel('모두 선택', {exact: true}).check();
  await expect(page.locator('#selection-count')).toHaveText('1개 선택됨');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});


test('tablet details overlay closes without clipping the view controls', async ({page}) => {
  await page.setViewportSize({width: 768, height: 844});
  const listBefore = await page.locator('#files').boundingBox();
  await page.locator('#files .row').filter({hasText: 'notes.txt'}).click();
  await page.getByLabel('세부 정보 창', {exact: true}).click();
  await expect(page.locator('#details')).toBeVisible();
  expect((await page.locator('#files').boundingBox()).width).toBe(listBefore.width);
  const close = await page.getByLabel('세부 정보 닫기').boundingBox();
  await page.mouse.click(close.x + close.width / 2, close.y + close.height / 2);
  await expect(page.locator('#details')).toBeHidden();
  const view = await page.locator('#btnView').boundingBox();
  await page.mouse.click(view.x + view.width / 2, view.y + view.height / 2);
  await expect(page.locator('#menuView')).toBeVisible();
  await page.screenshot({path: `${shots}/tablet-view-menu.png`});
});
