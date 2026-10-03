import assert from 'node:assert/strict';
import {test} from 'node:test';
import {ViewContinuity} from '../app/client/shared/scripts/ViewContinuity.js';

/** @typedef {{dataset:{graphGroup:string},focused:boolean,focusOptions:FocusOptions|null,scrollOptions:ScrollIntoViewOptions|null,closest:(selector:string)=>KeyboardTab|null,focus:(options:FocusOptions)=>void,scrollIntoView:(options:ScrollIntoViewOptions)=>void}} KeyboardTab */
/** @typedef {{first:KeyboardTab,last:KeyboardTab,listeners:Map<string,(event:Event)=>void>,selected:string[],watcher:ViewContinuity}} KeyboardFixture */
/** @typedef {{key:string,target:object,prevented:boolean,preventDefault:()=>void}} KeyboardInput */

class ViewContinuityKeyboardTest {
  /** @type {string} */
  static SELECTOR = '[data-graph-group]';
  /** @type {string} */
  static FIRST = 'fixture:first:birth';
  /** @type {string} */
  static LAST = 'fixture:last:birth';
  /** @type {string} */
  static FIRST_KEY = 'Home';

  /** @param {string} identifier @returns {KeyboardTab} */
  static tab(identifier) {
    return {
      dataset: {graphGroup: identifier},
      /** @param {string} selector @returns {KeyboardTab | null} */
      closest(selector) { return selector === ViewContinuityKeyboardTest.SELECTOR ? this : null; },
      /** @param {FocusOptions} options @returns {void} */
      focus(options) { this.focused = true; this.focusOptions = options; },
      /** @param {ScrollIntoViewOptions} options @returns {void} */
      scrollIntoView(options) { this.scrollOptions = options; },
      focused: false, focusOptions: null, scrollOptions: null
    };
  }

  /** @returns {KeyboardFixture} */
  static fixture() {
    class BoundaryElement {
      /** @param {object} values */
      constructor(values) { Object.assign(this, values); }
    }
    class BoundaryObserver {
      /** @param {Element} target @returns {void} */
      observe(target) {}
      /** @returns {void} */
      disconnect() {}
    }
    globalThis.HTMLElement = /** @type {typeof HTMLElement} */ (/** @type {unknown} */ (BoundaryElement));
    globalThis.MutationObserver = /** @type {typeof MutationObserver} */ (/** @type {unknown} */ (BoundaryObserver));
    globalThis.ResizeObserver = /** @type {typeof ResizeObserver} */ (/** @type {unknown} */ (BoundaryObserver));
    const first = /** @type {KeyboardTab} */ (/** @type {unknown} */ (new BoundaryElement(ViewContinuityKeyboardTest.tab(ViewContinuityKeyboardTest.FIRST))));
    const last = /** @type {KeyboardTab} */ (/** @type {unknown} */ (new BoundaryElement(ViewContinuityKeyboardTest.tab(ViewContinuityKeyboardTest.LAST))));
    /** @type {Map<string,(event:Event)=>void>} */
    const listeners = new Map();
    /** @type {string[]} */
    const selected = [];
    const view = {
      /** @param {string} name @param {(event: Event) => void} callback @returns {void} */
      addEventListener(name, callback) { listeners.set(name, callback); },
      /** @param {string} name @returns {void} */
      removeEventListener(name) { listeners.delete(name); },
      /** @param {string} selector @returns {object[]} */
      querySelectorAll(selector) { return selector === ViewContinuityKeyboardTest.SELECTOR ? [first, last] : []; }
    };
    const selection = {
      group: null, mode: 'cpu',
      /** @param {Event} event @param {string} identifier @returns {void} */
      showGraph(event, identifier) { selected.push(identifier); }
    };
    const watcher = new ViewContinuity(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), new AbortController().signal, /** @returns {import('../app/client/models/ViewSelection.ts').ViewSelection} */ () => selection);
    watcher.start();
    return {first, last, listeners, selected, watcher};
  }

  /** @param {KeyboardFixture} fixture @param {string} key @param {object} target @returns {KeyboardInput} */
  static press(fixture, key, target) {
    const event = {
      key, target, prevented: false,
      /** @returns {void} */
      preventDefault() { this.prevented = true; }
    };
    const listener = fixture.listeners.get('keydown');
    assert.ok(listener);
    listener(/** @type {Event} */ (/** @type {unknown} */ (event)));
    return event;
  }
}

test('Home selects and focuses the first real graph tab without scrolling focus', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  const event = ViewContinuityKeyboardTest.press(fixture, ViewContinuityKeyboardTest.FIRST_KEY, fixture.last);
  assert.deepEqual(fixture.selected, [ViewContinuityKeyboardTest.FIRST]);
  assert.equal(fixture.first.focused, true);
  assert.deepEqual(fixture.first.focusOptions, {preventScroll: true});
  assert.equal(event.prevented, true);
  fixture.watcher.dispose();
});

test('End selects and focuses the last real graph tab', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  ViewContinuityKeyboardTest.press(fixture, 'End', fixture.first);
  assert.deepEqual(fixture.selected, [ViewContinuityKeyboardTest.LAST]);
  assert.equal(fixture.last.focused, true);
  fixture.watcher.dispose();
});

test('ArrowLeft wraps the first graph tab to the last', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  ViewContinuityKeyboardTest.press(fixture, 'ArrowLeft', fixture.first);
  assert.deepEqual(fixture.selected, [ViewContinuityKeyboardTest.LAST]);
  assert.equal(fixture.last.focused, true);
  fixture.watcher.dispose();
});

test('ArrowRight wraps the last graph tab to the first', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  ViewContinuityKeyboardTest.press(fixture, 'ArrowRight', fixture.last);
  assert.deepEqual(fixture.selected, [ViewContinuityKeyboardTest.FIRST]);
  assert.equal(fixture.first.focused, true);
  fixture.watcher.dispose();
});

test('an unrelated key preserves graph selection and browser keyboard behavior', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  const event = ViewContinuityKeyboardTest.press(fixture, 'Tab', fixture.first);
  assert.deepEqual(fixture.selected, []);
  assert.equal(fixture.first.focused, false);
  assert.equal(event.prevented, false);
  fixture.watcher.dispose();
});

test('navigation from outside a graph tab preserves selection', /** @returns {void} */ () => {
  const fixture = ViewContinuityKeyboardTest.fixture();
  const event = ViewContinuityKeyboardTest.press(fixture, ViewContinuityKeyboardTest.FIRST_KEY, {});
  assert.deepEqual(fixture.selected, []);
  assert.equal(event.prevented, false);
  fixture.watcher.dispose();
});
