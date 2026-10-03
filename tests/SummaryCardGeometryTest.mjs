import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

/** @typedef {{width: number, height: number}} WindowSize */
/** @typedef {{left: number, top: number, right: number, bottom: number}} TextBounds */
/** @typedef {TextBounds & {name: string, radius: string}} ClippingAncestor */
/** @typedef {{text: string | null, lineHeight: string, bounds: TextBounds, clips: ClippingAncestor[]}} TextGeometry */
/** @typedef {{texts: TextGeometry[]}} CardGeometry */
/** @typedef {{size: WindowSize, cards: CardGeometry[]}} SummaryGeometry */
/** @typedef {{size: WindowSize, text: string | null, clip: ClippingAncestor, cropped: boolean, roundedEdge: boolean}} GeometryFailure */
class SummaryCardGeometryTest {
  /** @type {URL} */
  static ROOT = new URL('../', import.meta.url);
  /** @type {string} */
  static ORIGIN = 'https://summary-card-offline.invalid';
  /** @returns {Promise<void>} */
  static async run() {
    const browser = await chromium.launch({executablePath: QaPathUtils.external('OBSERVATORY_BROWSER_EXECUTABLE')});
    /** @type {SummaryGeometry[]} */
    const results = [];
    try {
      const page = await browser.newPage({viewport: {width: 1920, height: 928}});
      await this.route(page);
      await page.goto(this.ORIGIN);
      await page.waitForFunction(/** @returns {boolean} */ () => document.querySelectorAll('.metric-card .w-card-body').length === 4);
      results.push(...await this.captureSizes(page));
    } finally { await browser.close(); }
    const failures = this.failures(results);
    writeFileSync(QaPathUtils.evidence('offline-ui-summary-card-geometry.json'), JSON.stringify({kind:'actual DuVay/template/CSS offline geometry; synthetic counters; NOT native',results,failures,success:failures.length===0},null,2));
    assert.deepEqual(failures, [], 'Summary text must fit every actual clipping ancestor, including rounded corners.');
    console.log('PASS: four actual DuVay summary cards; title/count/note clip bounds at 1920×928, 1240×850, 1000×700.');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async route(page) {
    await page.route('**/*', /** @param {import('playwright').Route} route @returns {Promise<void>} */ async route => {
      const url = new URL(route.request().url());
      assert.equal(url.origin, this.ORIGIN);
      if (url.pathname === '/') {
        const template = readFileSync(new URL('app/client/components/observatory/panel/tac.html', this.ROOT), 'utf8');
        const matched = template.match(/<section class="summary-grid[\s\S]*?<\/section>/);
        assert.ok(matched);
        const summary = matched[0].replace(/\{observatory\.model\.\w+\}/g, '6');
        return route.fulfill({contentType: 'text/html', body: '<link rel="stylesheet" href="/shared/assets/duvay/duvay.min.css"><link rel="stylesheet" href="/shared/styles/app.css"><script type="module" src="/shared/assets/duvay/duvay-wc.min.js"></script><div class="observatory"><aside class="sidebar"></aside><main class="main section-running">' + summary + '</main></div>'});
      }
      assert.match(url.pathname, /^\/shared\/(assets\/duvay|styles)\/[\w.-]+$/);
      return route.fulfill({contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript', body: readFileSync(new URL('app/client' + url.pathname, this.ROOT))});
    });
  }

  /** @param {import('playwright').Page} page @returns {Promise<SummaryGeometry[]>} */
  static async captureSizes(page) {
    /** @type {SummaryGeometry[]} */
    const results = [];
    for (const size of [{width:1920,height:928},{width:1240,height:850},{width:1000,height:700}]) {
      await page.setViewportSize(size);
      const cards = await page.evaluate(/** @returns {CardGeometry[]} */ () => [...document.querySelectorAll('.metric-card')].map(/** @param {Element} card @returns {CardGeometry} */ card => {
        const texts = [...card.querySelectorAll('.metric-label,strong,p')].map(/** @param {Element} element @returns {TextGeometry} */ element => {
          const range = document.createRange(); range.selectNodeContents(element);
          const text = range.getBoundingClientRect();
          /** @type {ClippingAncestor[]} */
          const ancestors = [];
          for (let parent = element.parentElement; parent && parent !== card.parentElement; parent = parent.parentElement) {
            const style = getComputedStyle(parent); const rect = parent.getBoundingClientRect();
            if (style.overflowX === 'hidden' || style.overflowY === 'hidden') ancestors.push({name:parent.className, radius:style.borderTopLeftRadius, left:rect.left, top:rect.top, right:rect.right, bottom:rect.bottom});
          }
          return {text:element.textContent, lineHeight:getComputedStyle(element).lineHeight, bounds:{left:text.left,top:text.top,right:text.right,bottom:text.bottom}, clips:ancestors};
        });
        return {texts};
      }));
      results.push({size, cards});
    }
    return results;
  }

  /** @param {SummaryGeometry[]} results @returns {GeometryFailure[]} */
  static failures(results) {
    return results.flatMap(/** @param {SummaryGeometry} result @returns {GeometryFailure[]} */ result => result.cards.flatMap(/** @param {CardGeometry} card @returns {GeometryFailure[]} */ card => card.texts.flatMap(/** @param {TextGeometry} text @returns {GeometryFailure[]} */ text => text.clips.flatMap(/** @param {ClippingAncestor} clip @returns {GeometryFailure[]} */ clip => {
      const b = text.bounds;
      const cropped = b.left < clip.left || b.top < clip.top || b.right > clip.right || b.bottom > clip.bottom;
      const roundedEdge = parseFloat(clip.radius) > 0 && (b.left < clip.left + parseFloat(clip.radius) || b.right > clip.right - parseFloat(clip.radius)) && (b.top < clip.top + parseFloat(clip.radius) || b.bottom > clip.bottom - parseFloat(clip.radius));
      return cropped || roundedEdge ? [{size:result.size,text:text.text,clip,cropped,roundedEdge}] : [];
    }))));
  }

}
await SummaryCardGeometryTest.run();
