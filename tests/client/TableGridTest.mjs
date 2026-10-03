import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

/** @typedef {{x: number, y: number, width: number}} CardRectangle */

class TableGridTest {
  /** @type {URL} */
  static ROOT = new URL('../../', import.meta.url);
  /** @type {string} */
  static CHROME = QaPathUtils.external('OBSERVATORY_BROWSER_EXECUTABLE');

  /** @returns {Promise<void>} */
  static async run() {
    const browser = await chromium.launch({ executablePath: this.CHROME });
    await this.checkAndClose(browser);
  }

  /** @param {import('playwright').Browser} browser @returns {Promise<void>} */
  static async checkAndClose(browser) {
    try {
      const page = await browser.newPage({ viewport: { width: 1240, height: 850 } });
      await page.route('**/*', /** @param {import('playwright').Route} route @returns {Promise<void>} */ route => route.abort());
      await page.setContent(this.document());
      await this.#cardsOnly(page);
      await this.normal(page);
      await page.setViewportSize({ width: 1680, height: 1000 });
      await this.wide(page);
      await page.setViewportSize({ width: 1000, height: 700 });
      await this.small(page);
      await page.setViewportSize({ width: 900, height: 700 });
      await this.small(page);
      console.log('PASS: actual Table template/CSS has four equal cards at1680, two at1240 and one at1000/900; fixture only, no HTTP server or native acceptance claim.');
    } finally {
      await browser.close();
    }
  }

  /** @returns {string} */
  static document() {
    const styles = readFileSync(new URL('app/client/shared/styles/app.css', this.ROOT), 'utf8');
    const components = readFileSync(new URL('app/client/shared/assets/duvay/duvay.min.css', this.ROOT), 'utf8');
    const cards = this.card('Codex', 'codex') + this.card('Claude', 'claude') + this.card('Codex', 'codex-two') + this.card('Claude', 'claude-two');
    return '<!doctype html><html lang="en"><head><style>' + components + styles + '</style></head><body><w-container class="observatory"><aside class="sidebar">UI layout fixture only</aside><main class="main section-processes"><div class="workspace-grid"><div class="sessions-panel"><div class="session-list mode-table">' + cards + '</div></div></div></main></w-container></body></html>';
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #cardsOnly(page) {
    assert.equal(await page.locator('.table-branch > .session-row').count(), 4, 'Table keeps the responsive runtime cards.');
    assert.equal(await page.locator('.table-branch .tree-expansion,.table-branch .tree-node,.table-branch .process-table-rows').count(), 0, 'Table must not duplicate the process hierarchy below its cards.');
  }

  /** @param {string} family @param {string} name @returns {string} */
  static card(family, name) {
    const source = readFileSync(new URL('app/client/components/observatory/panel/tac.html', this.ROOT), 'utf8');
    const marker = '<loop :for="group of observatory.model.sessions">';
    const start = source.indexOf(marker) + marker.length;
    const end = source.indexOf('<logic :if="observatory.model.sessionCount === 0">', start);
    assert.ok(start >= marker.length && end > start, 'find real Table group template');
    return source.slice(start, end)
      .replace(/:class="'session-row ' \+ group.selected"/g, 'class="session-row"')
      .replace(/<\/?(?:if|for)\b[^>]*>/g, '')
      .replace(/\{group.family\}/g, family)
      .replace(/\{group.name\}/g, name)
      .replace(/\{group.avatar\}/g, family)
      .replace(/\{group.pid\}/g, '12345')
      .replace(/\{group.processCount\}/g, '4')
      .replace(/\{group.cpu\}/g, 'Unavailable')
      .replace(/\{group.memory\}/g, 'Unavailable')
      .replace(/\{group.download\}/g, 'Unavailable')
      .replace(/\{group.scopeLabel\}/g, 'Shared runtime')
      .replace(/\{group.hookLabel\}/g, 'No linked task events')
      .replace(/\{group.status\}/g, 'Process observed');
  }

  /** @param {import('playwright').Page} page @returns {Promise<CardRectangle[]>} */
  static async rectangles(page) {
    return page.locator('.table-branch').evaluateAll(/** @param {Element[]} cards @returns {CardRectangle[]} */ cards => cards.map(/** @param {Element} card @returns {CardRectangle} */ card => {
      const rectangle = card.getBoundingClientRect();
      return { x: rectangle.x, y: rectangle.y, width: rectangle.width };
    }));
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async normal(page) {
    const cards = await this.rectangles(page);
    assert.equal(cards.length, 4);
    assert.equal(cards[0].y, cards[1].y, 'different providers must share a Table row');
    assert.equal(cards[0].width, cards[1].width, 'Table cards have equal width');
    assert.ok(cards[1].x > cards[0].x, 'second Table card is beside the first');
    assert.ok(cards[2].y > cards[0].y, 'third medium Table card begins next row');
    assert.equal(await page.evaluate(/** @returns {boolean} */ () => document.documentElement.scrollWidth > innerWidth), false);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async wide(page) {
    const cards = await this.rectangles(page);
    assert.equal(cards.length, 4);
    assert.ok(cards.every(/** @param {CardRectangle} card @returns {boolean} */ card => card.y === cards[0].y && Math.abs(card.width - cards[0].width) < 1));
    assert.ok(cards[0].width >= 300, 'wide cards preserve useful reading width');
    assert.equal(await page.evaluate(/** @returns {boolean} */ () => document.documentElement.scrollWidth > innerWidth), false);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async small(page) {
    const cards = await this.rectangles(page);
    assert.equal(cards[0].x, cards[1].x, 'small Table cards use one column');
    assert.ok(cards[1].y > cards[0].y, 'second small card is below the first');
    assert.equal(await page.evaluate(/** @returns {boolean} */ () => document.documentElement.scrollWidth > innerWidth), false);
  }
}

await TableGridTest.run();
