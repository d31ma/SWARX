import assert from 'node:assert/strict';
import {readFileSync, mkdirSync, writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

/** @typedef {{package: string, version: string, license: string, sha256: string}} VendorPin */

export class ProvenanceVendorFixture {
  /** @type {Record<string, VendorPin>} */
  static #VENDORS = {
    '/shared/assets/duvay/duvay-wc.min.js': {package: 'DuVay', version: '26.35.01', license: 'MIT', sha256: 'bd252fed5bb8a90f4c4f9eb1ac8d948e8b5c9dcf88e35ccfe30915ba9d2960de'},
    '/shared/assets/duvay/duvay.min.js': {package: 'DuVay', version: '26.35.01', license: 'MIT', sha256: '50ca0fd49f2232e3520f9e3d1fe8efa1fec1948a69e1076d43ed649a09b80d5b'},
    '/shared/assets/chartjs/chart.umd.min.js': {package: 'Chart.js', version: '4.5.1', license: 'MIT', sha256: '48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a'}
  };

  /** @returns {{tac: {scripts: string[], scriptSources: Record<string, VendorPin>}}} */
  static manifest() {
    return {tac: {scripts: Object.keys(this.#VENDORS), scriptSources: structuredClone(this.#VENDORS)}};
  }

  /** @param {URL} project @param {URL} emitted @returns {void} */
  static copy(project, emitted) {
    this.#copyVendors(project, emitted);
  }

  /** @param {URL} project @param {URL} emitted @returns {void} */
  static #copyVendors(project, emitted) {
    for (const [relative, pin] of Object.entries(this.#VENDORS)) {
      this.#copyVendor(relative, pin, project, emitted);
    }
  }

  /** @param {string} relative @param {VendorPin} pin @param {URL} project @param {URL} emitted @returns {void} */
  static #copyVendor(relative, pin, project, emitted) {
    const bytes = readFileSync(new URL('../../../app/client' + relative, import.meta.url));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), pin.sha256, 'Fixture must capture the real pinned vendor bytes.');
    this.#put(new URL('app/client' + relative, project), bytes);
    this.#put(new URL(relative.slice(1), emitted), bytes);
  }

  /** @param {URL} file @param {Uint8Array} bytes @returns {void} */
  static #put(file, bytes) {
    mkdirSync(new URL('./', file), {recursive: true});
    writeFileSync(file, bytes);
  }
}
