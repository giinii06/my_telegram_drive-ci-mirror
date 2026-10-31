import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
const entries = [
  { type: "file", name: "a.jpg", path: "/a.jpg", size: 10, upload_time: 1 },
  { type: "file", name: "b.mp4", path: "/b.mp4", size: 20, upload_time: 2 },
  { type: "file", name: "notes.txt", path: "/notes.txt", size: 5, upload_time: 3 },
  { type: "folder", name: "sub", path: "/sub" },
  { type: "file", name: "d.png", path: "/d.png", size: 8, upload_time: 4 },
  { type: "file", name: "run.exe", path: "/run.exe", size: 9, upload_time: 5 },
];

test('gallery navigation, previews, settings, and move dialog', async ({ page }) => {
  await page.addInitScript(() => {
    HTMLMediaElement.prototype.play = () => Promise.resolve();
  });
  await page.route('http://tdrive.test/**', route => {
    const url = new URL(route.request().url());
    if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body: html });
    if (url.pathname === '/api/status') {
      return route.fulfill({ json: { ok: true, status: { drive_id: 7, drive_title: 'TDrive', current_path: '/', vault_configured: false, vault_unlocked: false } } });
    }
    if (url.pathname === '/api/list') {
      return route.fulfill({ json: { ok: true, path: url.searchParams.get('path') || '/', entries } });
    }
    return route.abort();
  });
  await page.goto('http://tdrive.test/?token=t');
  await expect(page.locator('#files .row')).toHaveCount(6);
  await page.evaluate(async entries => {
    const $ = id => window.document.getElementById(id);
    const assert = (condition, message) => { if (!condition) throw new Error(message); };
    const stageTag = () => ($('stage').firstChild || {}).tagName;
    assert(window.location.search.includes("path="), "URL carries path, got " + window.location.search);
    await window.load("/sub");
    assert(window.location.search.includes("path=%2Fsub"), "URL updates on navigate, got " + window.location.search);
    await window.load("/");
    assert(window.document.querySelectorAll("#files .row").length === 6, "6 rows rendered");
    await window.load("/sub");
    window.history.back();
  }, entries);
  await expect.poll(() => page.evaluate(() => window.state.cwd)).toBe('/');
  await page.evaluate(async entries => {
    const $ = id => window.document.getElementById(id);
    const assert = (condition, message) => { if (!condition) throw new Error(message); };
    const stageTag = () => ($('stage').firstChild || {}).tagName;
    assert(window.state.cwd === "/", "browser back returns to /, got " + window.state.cwd);
    assert(window.location.search.includes("path=%2F"), "URL back to /, got " + window.location.search);
    window.preview(entries[1]);
    assert(stageTag() === "VIDEO", "b.mp4 shows VIDEO, got " + stageTag());
    assert($("pvcap").textContent.includes("b.mp4") && $("pvcap").textContent.includes("2 / 4"), "caption 'b.mp4 (2 / 4)', got " + $("pvcap").textContent);
    assert($("dl").href.includes("b.mp4") && $("dl").href.includes("download=1"), "download link follows item");
    window.document.dispatchEvent(new window.KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    assert(stageTag() === "IMG" && $("pvcap").textContent.includes("d.png"), "next shows d.png IMG, got " + stageTag() + " / " + $("pvcap").textContent);
    window.pvStep(1);
    assert(stageTag() === "PRE", "notes.txt shows PRE, got " + stageTag());
    window.pvStep(1);
    assert(stageTag() === "IMG" && $("pvcap").textContent.includes("a.jpg") && $("pvcap").textContent.includes("1 / 4"), "wrap to a.jpg (1/4), got " + $("pvcap").textContent);
    window.document.dispatchEvent(new window.KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    assert($("pvcap").textContent.includes("notes.txt"), "left wraps to notes.txt, got " + $("pvcap").textContent);
    $("closePv").onclick();
    assert(!$("preview").classList.contains("open"), "preview closed");
    window.preview(entries[5]);
    assert($("stage").textContent.includes("지원하지 않는 형식"), "exe shows unsupported message");
    assert($("pvPrev").disabled && $("pvNext").disabled, "arrows disabled for single item");
    await window.openSettings();
    assert($("modalWrap").classList.contains("open"), "settings modal opens");
    assert($("modal").textContent.includes("접속 토큰"), "settings shows token section");
    assert($("modal").textContent.includes("TDrive"), "settings shows drive title");
    window.closeModal();
    window.state.sel = ["/a.jpg"];
    await window.openMoveTo();
    assert($("modalWrap").classList.contains("open"), "move dialog opens");
    assert($("movelist").textContent.includes("sub"), "move dialog lists sub folder");
    window.closeModal();
  }, entries);
});
