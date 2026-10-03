import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {test} from 'node:test';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

class CommandResourceLayoutTest {
  /** @type {string} */
  static BROWSER = QaPathUtils.external('OBSERVATORY_BROWSER_EXECUTABLE');
  /** @type {string} */
  static CSS = fs.readFileSync(new URL('../app/client/shared/styles/app.css', import.meta.url), 'utf8');

  /** @param {number} width @returns {Promise<void>} */
  static async verify(width) {
    const browser = await chromium.launch({executablePath: this.BROWSER, headless: true});
    try {
      const page = await browser.newPage({viewport: {width: 1240, height: 850}});
      await page.setContent(this.document(width));
      const bounds = await page.locator('.command-resource').evaluate(/** @param {Element} element @returns {{width: number, available: number}} */ element => {
        const parent = element.parentElement;
        if (parent === null) throw new Error('The resource card must have an agent-card parent.');
        return {width: element.getBoundingClientRect().width, available: parent.getBoundingClientRect().width};
      });
      assert.ok(bounds.width >= bounds.available - 1, 'A command resource card uses the available agent-card width.');
      assert.equal(await page.locator('.command-resource').evaluate(/** @param {Element} element @returns {boolean} */ element => element.scrollWidth > element.clientWidth), false);
    } finally {
      await browser.close();
    }
  }

  /** @param {number} width @returns {string} */
  static document(width) {
    return '<style>' + this.CSS + '</style><article style="width:' + width + 'px;padding:20px" class="logical-agent-row"><div class="agent-resources"><section class="command-resource"><h4>Synthetic layout fixture · linked command process</h4><p class="command-resource-scope">PID 123 · command root only · no whole-agent total</p><dl class="command-resource-metrics"><div><dt>CPU</dt><dd>0.0%</dd></div><div><dt>RSS</dt><dd>1.1 MiB</dd></div><div><dt>Disk read</dt><dd>0 B/s</dd></div><div><dt>Disk write</dt><dd>0 B/s</dd></div><div><dt>Network</dt><dd>Unavailable</dd></div></dl><p class="command-resource-trust">Caller-reported command link; OS identity matched</p></section></div></article>';
  }
}

test('linked resource card uses normal agent-card width', /** @returns {Promise<void>} */ async () => {
  await CommandResourceLayoutTest.verify(476);
});

test('linked resource card uses small-window agent-card width', /** @returns {Promise<void>} */ async () => {
  await CommandResourceLayoutTest.verify(756);
});
