import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

/** @typedef {{x: number, y: number, width: number}} CardRectangle */
/** @typedef {{family: string, harness: string, name: string}} CardSession */
/** @typedef {{open: string, close: string}} TemplateTag */

class TableGridTest {
  /** @type {URL} */
  static ROOT = new URL('../../', import.meta.url);
  /** @type {string} */
  static CHROME = QaPathUtils.external('OBSERVATORY_BROWSER_EXECUTABLE');
  /** @type {string} */
  static COMPONENTS = 'app/client/components/observatory/';
  /** @type {string} */
  static CARD_LOOP = '<loop :for="session of observatory.model.activeSessions">';
  /** @type {string} */
  static EMPTY_FEED = '<logic :if="observatory.model.activeSessions.length === 0">';
  /** @type {string[]} */
  static OPTIONAL_CARD_PARTS = ['<logic :if="session.loads.length">', '<logic :if="session.background.length">', '<logic :if="session.collapsible">'];
  /** @type {TemplateTag} */
  static LOGIC = {open: '<logic', close: '</logic>'};
  /** @type {TemplateTag} */
  static LOOP = {open: '<loop', close: '</loop>'};
  /** @type {string} */
  static CARD = '.session-grid > .session-card';
  /** @type {string} */
  static PANEL = '.session-panel';
  /** @type {string} */
  static SELECTED_SESSION = 'codex-two';
  /** @type {CardSession[]} */
  static SESSIONS = [{family: 'Codex', harness: 'Codex', name: 'codex'}, {family: 'Claude', harness: 'Claude Code', name: 'claude'},
    {family: 'Codex', harness: 'Codex', name: 'codex-two'}, {family: 'Claude', harness: 'Claude Code', name: 'claude-two'}];
  /** @type {{width: number, height: number}} */
  static NORMAL = {width: 1240, height: 850};
  /** @type {{width: number, height: number}} */
  static WIDE = {width: 1680, height: 1000};
  /** @type {{width: number, height: number}} */
  static NARROW = {width: 900, height: 700};

  /** @returns {Promise<void>} */
  static async run() {
    const browser = await chromium.launch({ executablePath: this.CHROME });
    await this.checkAndClose(browser);
  }

  /** @param {import('playwright').Browser} browser @returns {Promise<void>} */
  static async checkAndClose(browser) {
    try {
      const page = await browser.newPage({ viewport: this.NORMAL });
      await page.route('**/*', /** @param {import('playwright').Route} route @returns {Promise<void>} */ route => route.abort());
      await page.setContent(this.document(false));
      await this.#cardsOnly(page);
      await this.list(page, 3);
      await page.setViewportSize(this.WIDE);
      await this.list(page, 4);
      await this.wide(page);
      await page.setViewportSize({ width: 1000, height: 700 });
      await this.list(page, 2);
      await page.setViewportSize(this.NARROW);
      await this.list(page, 2);
      await page.setContent(this.document(true));
      await page.setViewportSize(this.WIDE);
      await this.columnFlow(page);
      await page.setViewportSize(this.NORMAL);
      await this.oneColumn(page);
      await page.setViewportSize(this.NARROW);
      await this.hiddenBesideNarrowPanel(page);
      console.log('PASS: actual session card template/CSS has four equal cards at 1680, three at 1240 and two at 1000/900; beside the open panel two packed columns at 1680, one at 1240 and hidden at 900; fixture only, no HTTP server or native acceptance claim.');
    } finally {
      await browser.close();
    }
  }

  /** @param {string} name @returns {string} */
  static source(name) {
    return readFileSync(new URL(this.COMPONENTS + name, this.ROOT), 'utf8');
  }

  /** @param {string} component @returns {string} */
  static scopedStyles(component) {
    return '@scope (tachyon-component[data-tac-scope="observatory-' + component + '"]) to (tachyon-component[data-tac-scope]) {\n' + this.source(component + '/tac.css') + '\n}\n';
  }

  /** @param {boolean} open @returns {string} */
  static document(open) {
    const styles = readFileSync(new URL('app/client/shared/styles/app.css', this.ROOT), 'utf8');
    const components = readFileSync(new URL('app/client/shared/assets/duvay/duvay.min.css', this.ROOT), 'utf8');
    const scoped = this.scopedStyles('panel') + this.scopedStyles('running') + this.scopedStyles('session');
    const state = open ? ' has-session' : '';
    const panel = open ? '<tachyon-component data-tac-scope="observatory-session"><aside id="session-panel" class="session-panel is-open"><div class="session-panel-body">Session panel layout fixture only</div></aside></tachyon-component>' : '';
    return '<!doctype html><html lang="en"><head><style>' + components + styles + scoped + '</style></head><body>'
      + '<tachyon-component data-tac-scope="observatory-panel"><w-container fluid class="observatory"><main class="main"><section class="workspace-grid">'
      + '<section class="logical-panel' + state + '"><tachyon-component data-tac-scope="observatory-running">' + this.sessionArea(open) + '</tachyon-component>' + panel
      + '</section></section></main></w-container></tachyon-component></body></html>';
  }

  /** @param {boolean} open @returns {string} */
  static sessionArea(open) {
    const source = this.withoutBlock(this.source('running/tac.html'), this.EMPTY_FEED, this.LOGIC);
    const start = source.indexOf(this.CARD_LOOP);
    assert.ok(start > 0, 'find real session card template');
    const end = this.blockEnd(source, start, this.LOOP);
    const template = source.slice(start + this.CARD_LOOP.length, end);
    let cards = '';
    for (const session of this.SESSIONS) {
      cards += this.card(template, session, open);
    }
    const rendered = this.plain(source.slice(0, start) + cards + source.slice(end + this.LOOP.close.length))
      .replace(/:class="'session-area'[^"]*"/, 'class="session-area' + (open ? ' has-session' : '') + '"')
      .replace(/:class="'session-grid'[^"]*"/, 'class="session-grid"')
      .replace(/\{observatory\.model\.logicalNotice\}/g, 'Synthetic layout notice.');
    assert.doesNotMatch(rendered, /[{}]|<\/?loop|<\/?logic|\s:[\w-]+=|\son:/, 'Every template binding in the fixture is resolved.');
    return rendered;
  }

  /** @param {string} template @param {CardSession} session @param {boolean} open @returns {string} */
  static card(template, session, open) {
    const selected = open && session.name === this.SELECTED_SESSION ? ' is-selected' : '';
    let kept = template;
    for (const marker of this.OPTIONAL_CARD_PARTS) {
      kept = this.withoutBlock(kept, marker, this.LOGIC);
    }
    return this.plain(kept)
      .replace(/:class="'session-card'[^"]*"/, 'class="session-card' + selected + '" data-session="' + session.family + ':' + session.name + '"')
      .replace(/:class="'(agent-role|agent-tool)'[^"]*"/g, 'class="$1"')
      .replace(/\{session\.title\}/g, 'Synthetic ' + session.name + ' session')
      .replace(/\{session\.harness\}/g, session.harness)
      .replace(/\{session\.countLabel\}/g, '1 Agent · 0 Sub-agents · 1 tool running')
      .replace(/\{row\.roleLabel\}/g, 'Agent')
      .replace(/\{row\.toolLabel\}/g, 'fixture-action')
      .replace(/\{row\.actionDetail\}/g, 'Action details not reported');
  }

  /** @param {string} text @returns {string} */
  static plain(text) {
    return text
      .replace(/<\/?loop\b[^>]*>/g, '')
      .replace(/<\/?logic\b[^>]*>/g, '')
      .replace(/\son:[\w-]+="[^"]*"/g, '')
      .replace(/\s:(?!class=)[\w-]+="[^"]*"/g, '');
  }

  /** @param {string} text @param {string} marker @param {TemplateTag} tag @returns {string} */
  static withoutBlock(text, marker, tag) {
    const start = text.indexOf(marker);
    assert.ok(start >= 0, 'find template block ' + marker);
    return text.slice(0, start) + text.slice(this.blockEnd(text, start, tag) + tag.close.length);
  }

  /** @param {string} text @param {number} start @param {TemplateTag} tag @returns {number} */
  static blockEnd(text, start, tag) {
    let depth = 0;
    let index = start;
    while (depth >= 0) {
      const open = text.indexOf(tag.open, index + 1);
      const close = text.indexOf(tag.close, index + 1);
      assert.ok(close > 0, 'template block closes after ' + start);
      const nested = open > 0 && open < close;
      depth += nested ? 1 : -1;
      index = nested ? open : close;
    }
    return index;
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #cardsOnly(page) {
    assert.equal(await page.locator(this.CARD).count(), 4, 'The session grid keeps one card per active session.');
    assert.equal(await page.locator('.session-card .agent-row').count(), 4, 'Each card keeps its compact agent row.');
    assert.equal(await page.locator('.session-card .graph-node,.session-card .process-tree-panel,.session-card .history-card,.session-card .action-history-row').count(), 0, 'Cards must not duplicate the process tree, charts or activity of the session panel.');
  }

  /** @param {import('playwright').Page} page @returns {Promise<CardRectangle[]>} */
  static async rectangles(page) {
    return page.locator(this.CARD).evaluateAll(/** @param {Element[]} cards @returns {CardRectangle[]} */ cards => cards.map(/** @param {Element} card @returns {CardRectangle} */ card => {
      const rectangle = card.getBoundingClientRect();
      return { x: rectangle.x, y: rectangle.y, width: rectangle.width };
    }));
  }

  /** @param {import('playwright').Page} page @returns {Promise<boolean>} */
  static async sidewaysOverflow(page) {
    return page.evaluate(/** @returns {boolean} */ () => document.documentElement.scrollWidth > innerWidth);
  }

  /** @param {import('playwright').Page} page @param {number} columns @returns {Promise<void>} */
  static async list(page, columns) {
    const cards = await this.rectangles(page);
    assert.equal(cards.length, 4);
    assert.ok(cards.every(/** @param {CardRectangle} card @returns {boolean} */ card => Math.abs(card.width - cards[0].width) < 1), 'Session cards have equal width.');
    const firstRow = cards.filter(/** @param {CardRectangle} card @returns {boolean} */ card => card.y === cards[0].y);
    assert.equal(firstRow.length, columns, 'expected ' + columns + ' session card column(s) at ' + page.viewportSize()?.width + ': ' + JSON.stringify(cards));
    assert.ok(cards[1].x > cards[0].x, 'different providers share a row; the second card is beside the first');
    assert.equal(await this.sidewaysOverflow(page), false);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async wide(page) {
    const cards = await this.rectangles(page);
    assert.ok(cards[0].width >= 300, 'wide cards preserve useful reading width');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async columnFlow(page) {
    const cards = await this.rectangles(page);
    assert.equal(cards.length, 4);
    assert.equal(cards[0].x, cards[1].x, 'beside the open panel, cards fill a column top to bottom: ' + JSON.stringify(cards));
    assert.ok(cards[1].y > cards[0].y, 'the second card is below the first in its column');
    assert.ok(cards[2].x > cards[0].x, 'the third card starts the next packed column');
    assert.equal(cards[2].y, cards[0].y, 'both packed columns start flush at the top');
    assert.equal(await page.locator('.session-card.is-selected').getAttribute('data-session'), 'Codex:' + this.SELECTED_SESSION);
    assert.equal(await this.sidewaysOverflow(page), false);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async oneColumn(page) {
    const cards = await this.rectangles(page);
    assert.ok(cards.every(/** @param {CardRectangle} card @returns {boolean} */ card => card.x === cards[0].x), 'beside the open panel at 1240 the cards use one column: ' + JSON.stringify(cards));
    assert.ok(cards[1].y > cards[0].y, 'the second card is below the first');
    const panel = await page.locator(this.PANEL).boundingBox();
    assert.ok(panel && panel.x >= cards[0].x + cards[0].width, 'the open panel is a column beside the cards, never an overlay');
    assert.equal(await this.sidewaysOverflow(page), false);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async hiddenBesideNarrowPanel(page) {
    assert.equal(await page.locator('.session-area').evaluate(/** @param {Element} area @returns {string} */ area => getComputedStyle(area).display), 'none', 'below 920 px the open panel takes the whole content width');
    const panel = await page.locator(this.PANEL).boundingBox();
    const container = await page.locator('.logical-panel').boundingBox();
    assert.ok(panel && container && panel.width > container.width * 0.9, 'the narrow panel spans the content width');
    assert.equal(await this.sidewaysOverflow(page), false);
  }
}

await TableGridTest.run();
