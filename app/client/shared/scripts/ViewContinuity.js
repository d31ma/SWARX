import { GraphRenderer } from './GraphRenderer.js';
import { ChartRenderer } from './ChartRenderer.js';

export class ViewContinuity {
  /** @type {string[]} */
  static #SCROLL_SELECTORS = ['.session-list', '.graph-scroll', '.graph-group-tabs', '.detail-scroll', '.logical-scroll', '.history-scroll', '.activity-scroll', '.runtime-overview'];
  /** @type {string} */
  static #GROUP_PREFIX = 'graph-group:';
  /** @type {string} */
  static #AGENT_PREFIX = 'agent:';
  /** @type {string} */
  static #COMMAND_PREFIX = 'command:';
  /** @type {string} */
  static #COMMAND_SELECTOR = '.command-descendants';
  /** @type {string} */
  static #BINDING_ATTRIBUTE = 'binding';
  /** @type {string} */
  static #GRAPH_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static #EXECUTION_SELECTOR = '.execution-history';
  /** @type {string} */
  static #ACTION_SELECTOR = '.action-history-list';
  /** @type {string} */
  static #SUMMARY_SELECTOR = 'summary';
  /** @type {string} */
  static #DETAIL_SELECTOR = '.process-details';
  /** @type {string} */
  static #LIMITATIONS_SELECTOR = '.limitations';
  /** @type {string} */
  static #CLICK_EVENT = 'click';
  /** @type {string} */
  static #KEY_EVENT = 'keydown';
  /** @type {string} */
  static #FOCUS_EVENT = 'focusin';
  /** @type {string} */
  static #TAB_SELECTOR = '[data-graph-group]';
  /** @type {string} */
  static #SCROLL_EVENT = 'scroll';
  /** @type {string} */
  static #TOGGLE_EVENT = 'toggle';
  /** @type {string} */
  static #ABORT_EVENT = 'abort';
  /** @type {string} */
  static #EMPTY_IDENTIFIER = '';
  /** @type {string} */
  static #FIT_ACTION = 'fit';
  /** @type {ScrollLogicalPosition} */
  static #NEAREST_SCROLL = 'nearest';
  /** @type {number} */
  static #NO_VISIBLE_HEIGHT = 0;
  /** @type {number} */
  static #RESTORE_MILLISECONDS = 0;
  /** @type {string} */
  static #PREVIOUS_KEY = 'ArrowLeft';
  /** @type {string} */
  static #NEXT_KEY = 'ArrowRight';
  /** @type {string} */
  static #FIRST_KEY = 'Home';
  /** @type {string} */
  static #LAST_KEY = 'End';
  /** @type {string[]} */
  static #NAVIGATION_KEYS = [ViewContinuity.#PREVIOUS_KEY, ViewContinuity.#NEXT_KEY, ViewContinuity.#FIRST_KEY, ViewContinuity.#LAST_KEY];
  /** @type {HTMLElement} */
  #view;
  /** @type {AbortSignal} */
  #lifecycle;
  /** @type {() => import('/models/ViewSelection.ts').ViewSelection} */
  #selection;
  /** @type {Map<string, {top: number, left: number}>} */
  #offsets = new Map();
  /** @type {Map<string, boolean>} */
  #expanded = new Map();
  /** @type {Map<string, WeakRef<HTMLDetailsElement>>} */
  #toggleElements = new Map();
  /** @type {ReturnType<typeof setTimeout> | null} */
  #timer = null;
  /** @type {string} */
  #focusedGraph = ViewContinuity.#EMPTY_IDENTIFIER;
  /** @type {(event: Event) => void} */
  #scroll = /** @param {Event} event @returns {void} */ event => this.#rememberScroll(event);
  /** @type {(event: Event) => void} */
  #toggle = /** @param {Event} event @returns {void} */ event => this.#rememberToggle(event);
  /** @type {(event: Event) => void} */
  #click = /** @param {Event} event @returns {void} */ event => this.#pick(event);
  /** @type {(event: Event) => void} */
  #key = /** @param {Event} event @returns {void} */ event => this.#navigate(/** @type {KeyboardEvent} */ (event));
  /** @type {(event: Event) => void} */
  #focus = /** @param {Event} event @returns {void} */ event => this.#rememberFocus(event);
  /** @type {ResizeObserver} */
  #resize = new ResizeObserver(/** @returns {void} */ () => this.#schedule());
  /** @type {() => void} */
  #abort = /** @returns {void} */ () => this.dispose();
  /** @type {MutationObserver} */
  #observer = new MutationObserver(/** @returns {void} */ () => this.#schedule());

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @param {() => import('/models/ViewSelection.ts').ViewSelection} selection */
  constructor(view, lifecycle, selection) {
    this.#view = view;
    this.#lifecycle = lifecycle;
    this.#selection = selection;
  }

  /** @returns {void} */
  start() {
    this.#view.addEventListener(ViewContinuity.#CLICK_EVENT, this.#click);
    this.#view.addEventListener(ViewContinuity.#KEY_EVENT, this.#key);
    this.#view.addEventListener(ViewContinuity.#FOCUS_EVENT, this.#focus);
    this.#resize.observe(this.#view);
    this.#view.addEventListener(ViewContinuity.#SCROLL_EVENT, this.#scroll, {capture: true, passive: true});
    this.#view.addEventListener(ViewContinuity.#TOGGLE_EVENT, this.#toggle, true);
    this.#observer.observe(this.#view, {childList: true, subtree: true});
    this.#lifecycle.addEventListener(ViewContinuity.#ABORT_EVENT, this.#abort, {once: true});
  }

  /** @returns {void} */
  dispose() {
    this.#observer.disconnect();
    this.#resize.disconnect();
    this.#view.removeEventListener(ViewContinuity.#KEY_EVENT, this.#key);
    this.#view.removeEventListener(ViewContinuity.#FOCUS_EVENT, this.#focus);
    this.#cancelRestore();
    this.#view.removeEventListener(ViewContinuity.#SCROLL_EVENT, this.#scroll, true);
    this.#view.removeEventListener(ViewContinuity.#TOGGLE_EVENT, this.#toggle, true);
    this.#view.removeEventListener(ViewContinuity.#CLICK_EVENT, this.#click);
    this.#lifecycle.removeEventListener(ViewContinuity.#ABORT_EVENT, this.#abort);
    this.#toggleElements.clear();
    ChartRenderer.dispose();
  }

  /** @returns {void} */
  #cancelRestore() {
    if (this.#timer == null) {
      return;
    }
    clearTimeout(this.#timer);
    this.#timer = null;
  }

  /** @returns {void} */
  #schedule() {
    if (this.#timer != null) {
      return;
    }
    this.#timer = setTimeout(/** @returns {void} */ () => this.#restore(), ViewContinuity.#RESTORE_MILLISECONDS);
  }

  /** @returns {void} */
  #restore() {
    this.#timer = null;
    this.#restoreExpanded();
    const selection = this.#selection();
    this.#runtimeSelection(selection);
    GraphRenderer.draw(this.#view, selection);
    this.#restoreGraphFocus();
    ChartRenderer.draw(this.#view, selection);
    this.#restoreOffsets();
  }

  /** @param {Event} event @returns {void} */
  #rememberScroll(event) {
    const element = this.#target(event);
    const recording = this.#recording(element);
    this.#saveScrollWhenAllowed(element, recording);
  }

  /** @param {Event} event @returns {HTMLElement | null} */
  #target(event) {
    if (event.target instanceof HTMLElement) {
      return event.target;
    }
    return null;
  }

  /** @param {HTMLElement | null} element @returns {boolean} */
  #recording(element) {
    return [element !== null, element?.isConnected !== false].every(Boolean);
  }

  /** @param {HTMLElement | null} element @param {boolean} recording @returns {void} */
  #saveScrollWhenAllowed(element, recording) {
    if (!recording) {
      return;
    }
    this.#saveScroll(/** @type {HTMLElement} */ (element));
  }

  /** @param {HTMLElement} element @returns {void} */
  #saveScroll(element) {
    this.#saveGroupScroll(element);
    this.#saveActionScroll(element);
    this.#savePanelScroll(element);
  }

  /** @param {HTMLElement} element @returns {void} */
  #saveActionScroll(element) {
    if (!element.matches(ViewContinuity.#ACTION_SELECTOR)) {
      return;
    }
    this.#storeActionScroll(element);
  }

  /** @param {HTMLElement} element @returns {void} */
  #storeActionScroll(element) {
    const identifier = CSS.escape(this.#identifier(element.dataset.session));
    const selector = ViewContinuity.#ACTION_SELECTOR + '[data-session="' + identifier + '"]';
    this.#offsets.set(selector, {top: element.scrollTop, left: element.scrollLeft});
  }

  /** @param {HTMLElement} element @returns {void} */
  #saveGroupScroll(element) {
    if (!element.matches(ViewContinuity.#GRAPH_SELECTOR)) {
      return;
    }
    this.#offsets.set(ViewContinuity.#GROUP_PREFIX + element.dataset.group, {top: element.scrollTop, left: element.scrollLeft});
  }

  /** @param {HTMLElement} element @returns {void} */
  #savePanelScroll(element) {
    for (const selector of ViewContinuity.#SCROLL_SELECTORS) {
      this.#saveMatchingScroll(element, selector);
    }
  }

  /** @param {HTMLElement} element @param {string} selector @returns {void} */
  #saveMatchingScroll(element, selector) {
    const overflow = [element.scrollHeight > element.clientHeight, element.scrollWidth > element.clientWidth].some(Boolean);
    const visibleOverflow = [element.matches(selector), element.clientHeight > ViewContinuity.#NO_VISIBLE_HEIGHT, overflow].every(Boolean);
    if (!visibleOverflow) {
      return;
    }
    this.#offsets.set(selector, {top: element.scrollTop, left: element.scrollLeft});
  }

  /** @param {Event} event @returns {void} */
  #rememberToggle(event) {
    const element = this.#target(event);
    const recording = this.#recording(element);
    this.#saveToggleWhenAllowed(element, recording);
  }

  /** @param {HTMLElement | null} element @param {boolean} recording @returns {void} */
  #saveToggleWhenAllowed(element, recording) {
    if (!recording) {
      return;
    }
    this.#saveToggle(/** @type {HTMLDetailsElement} */ (element));
  }

  /** @param {HTMLDetailsElement} element @returns {void} */
  #saveToggle(element) {
    this.#saveMatchingToggle(element, ViewContinuity.#EXECUTION_SELECTOR, ViewContinuity.#AGENT_PREFIX + element.dataset.agent);
    this.#saveMatchingToggle(element, ViewContinuity.#COMMAND_SELECTOR, ViewContinuity.#COMMAND_PREFIX + element.dataset.binding);
    this.#saveMatchingToggle(element, ViewContinuity.#DETAIL_SELECTOR, ViewContinuity.#DETAIL_SELECTOR);
    this.#saveMatchingToggle(element, ViewContinuity.#LIMITATIONS_SELECTOR, ViewContinuity.#LIMITATIONS_SELECTOR);
  }

  /** @param {HTMLDetailsElement} element @param {string} selector @param {string} key @returns {void} */
  #saveMatchingToggle(element, selector, key) {
    const recording = [element.matches(selector), this.#canSaveToggle(element, key)].every(Boolean);
    this.#storeToggleWhenAllowed(element, key, element.open, recording);
  }

  /** @param {HTMLDetailsElement} element @param {string} key @returns {boolean} */
  #canSaveToggle(element, key) {
    return [!this.#expanded.has(key), this.#toggleElements.get(key)?.deref() === element].some(Boolean);
  }

  /** @param {HTMLDetailsElement} element @param {string} key @param {boolean} open @param {boolean} recording @returns {void} */
  #storeToggleWhenAllowed(element, key, open, recording) {
    if (!recording) {
      return;
    }
    this.#expanded.set(key, open);
    this.#toggleElements.set(key, new WeakRef(element));
  }

  /** @returns {void} */
  #restoreOffsets() {
    for (const [key, offset] of this.#offsets) {
      this.#restoreOffset(key, offset);
    }
  }

  /** @param {string} key @param {{top: number, left: number}} offset @returns {void} */
  #restoreOffset(key, offset) {
    const element = this.#findScroll(key);
    const normalized = this.#normalizeScroll(element);
    this.#setOffset(normalized, offset);
  }

  /** @param {string} key @returns {HTMLElement | null | undefined} */
  #findScroll(key) {
    if (key.startsWith(ViewContinuity.#GROUP_PREFIX)) {
      return this.#findGroup(key.slice(ViewContinuity.#GROUP_PREFIX.length));
    }
    return this.#view.querySelector(key);
  }

  /** @param {string} identifier @returns {HTMLElement | undefined} */
  #findGroup(identifier) {
    return this.#elements(ViewContinuity.#GRAPH_SELECTOR).find(
      /** @param {HTMLElement} element @returns {boolean} */
      element => element.dataset.group === identifier
    );
  }

  /** @param {string} selector @returns {HTMLElement[]} */
  #elements(selector) {
    return Array.from(this.#view.querySelectorAll(selector),
      /** @param {Element} element @returns {HTMLElement} */
      element => /** @type {HTMLElement} */ (element)
    );
  }

  /** @param {HTMLElement | null | undefined} element @returns {HTMLElement | null} */
  #normalizeScroll(element) {
    return element ?? null;
  }

  /** @param {HTMLElement | null} element @param {{top: number, left: number}} offset @returns {void} */
  #setOffset(element, offset) {
    if (!element) {
      return;
    }
    element.scrollTop = offset.top;
    element.scrollLeft = offset.left;
  }

  /** @returns {void} */
  #restoreExpanded() {
    for (const [key, open] of this.#expanded) {
      this.#restoreToggle(key, open);
    }
  }

  /** @param {string} key @param {boolean} open @returns {void} */
  #restoreToggle(key, open) {
    const element = this.#findToggle(key);
    this.#adoptToggle(element, key);
    this.#setOpen(element, open);
  }

  /** @param {HTMLDetailsElement | null} element @param {string} key @returns {void} */
  #adoptToggle(element, key) {
    if (element === null) {
      return;
    }
    this.#toggleElements.set(key, new WeakRef(element));
  }

  /** @param {string} key @returns {HTMLDetailsElement | null} */
  #findToggle(key) {
    const agent = this.#findPrefixedToggle(key, ViewContinuity.#AGENT_PREFIX, ViewContinuity.#EXECUTION_SELECTOR, 'agent');
    const command = this.#findPrefixedToggle(key, ViewContinuity.#COMMAND_PREFIX, ViewContinuity.#COMMAND_SELECTOR, ViewContinuity.#BINDING_ATTRIBUTE);
    return this.#chooseToggle(agent, command, key);
  }

  /** @param {string} key @param {string} prefix @param {string} selector @param {string} attribute @returns {HTMLDetailsElement | null | undefined} */
  #findPrefixedToggle(key, prefix, selector, attribute) {
    if (!key.startsWith(prefix)) {
      return null;
    }
    return this.#findDetail(selector, attribute, key.slice(prefix.length));
  }

  /** @param {string} selector @param {string} attribute @param {string} identifier @returns {HTMLDetailsElement | undefined} */
  #findDetail(selector, attribute, identifier) {
    return /** @type {HTMLDetailsElement | undefined} */ (this.#elements(selector).find(
      /** @param {HTMLElement} element @returns {boolean} */
      element => element.dataset[attribute] === identifier
    ));
  }

  /** @param {HTMLDetailsElement | null | undefined} agent @param {HTMLDetailsElement | null | undefined} command @param {string} key @returns {HTMLDetailsElement | null} */
  #chooseToggle(agent, command, key) {
    const found = this.#firstToggle(agent, command);
    const fallback = this.#plainToggle(key);
    return this.#toggleFallback(found, fallback);
  }

  /** @param {HTMLDetailsElement | null | undefined} agent @param {HTMLDetailsElement | null | undefined} command @returns {HTMLDetailsElement | null | undefined} */
  #firstToggle(agent, command) {
    return agent ?? command;
  }

  /** @param {HTMLDetailsElement | null | undefined} found @param {HTMLDetailsElement | null} fallback @returns {HTMLDetailsElement | null} */
  #toggleFallback(found, fallback) {
    return found ?? fallback;
  }

  /** @param {string} key @returns {HTMLDetailsElement | null} */
  #plainToggle(key) {
    if (!key.startsWith('.')) {
      return null;
    }
    return this.#view.querySelector(key);
  }

  /** @param {HTMLDetailsElement | null} element @param {boolean} open @returns {void} */
  #setOpen(element, open) {
    const changed = this.#needsOpenChange(element, open);
    this.#changeOpen(element, open, changed);
  }

  /** @param {HTMLDetailsElement | null} element @param {boolean} open @returns {boolean} */
  #needsOpenChange(element, open) {
    return [element !== null, element?.open !== open].every(Boolean);
  }

  /** @param {HTMLDetailsElement | null} element @param {boolean} open @param {boolean} changed @returns {void} */
  #changeOpen(element, open, changed) {
    if (!changed) {
      return;
    }
    /** @type {HTMLDetailsElement} */ (element).open = open;
  }

  /** @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #runtimeSelection(selection) {
    const element = this.#view.querySelector('.runtime-selector select');
    const identifier = this.#identifier(selection.model?.selected);
    this.#setRuntime(/** @type {HTMLSelectElement | null} */ (element), identifier);
  }

  /** @param {string | undefined} identifier @returns {string} */
  #identifier(identifier) {
    return identifier ?? ViewContinuity.#EMPTY_IDENTIFIER;
  }

  /** @param {HTMLSelectElement | null} element @param {string} identifier @returns {void} */
  #setRuntime(element, identifier) {
    if (!element) {
      return;
    }
    element.value = identifier;
  }

  /** @param {Event} event @returns {void} */
  #pick(event) {
    const target = this.#target(event);
    this.#rememberSummary(target, event.defaultPrevented);
    this.#pickTarget(event, target);
  }

  /** @param {HTMLElement | null} target @param {boolean} cancelled @returns {void} */
  #rememberSummary(target, cancelled) {
    const element = this.#summaryParent(target);
    this.#saveSummaryToggle(element, ViewContinuity.#EXECUTION_SELECTOR, ViewContinuity.#AGENT_PREFIX + element?.dataset.agent, cancelled);
    this.#saveSummaryToggle(element, ViewContinuity.#COMMAND_SELECTOR, ViewContinuity.#COMMAND_PREFIX + element?.dataset.binding, cancelled);
    this.#saveSummaryToggle(element, ViewContinuity.#DETAIL_SELECTOR, ViewContinuity.#DETAIL_SELECTOR, cancelled);
    this.#saveSummaryToggle(element, ViewContinuity.#LIMITATIONS_SELECTOR, ViewContinuity.#LIMITATIONS_SELECTOR, cancelled);
  }

  /** @param {HTMLElement | null} target @returns {HTMLDetailsElement | null} */
  #summaryParent(target) {
    return /** @type {HTMLDetailsElement | null} */ (target?.closest(ViewContinuity.#SUMMARY_SELECTOR)?.parentElement ?? null);
  }

  /** @param {HTMLDetailsElement | null} element @param {string} selector @param {string} key @param {boolean} cancelled @returns {void} */
  #saveSummaryToggle(element, selector, key, cancelled) {
    const recording = [element?.matches(selector) === true, element?.isConnected !== false, !cancelled].every(Boolean);
    if (!recording) {
      return;
    }
    const details = /** @type {HTMLDetailsElement} */ (element);
    this.#expanded.set(key, !details.open);
    this.#toggleElements.set(key, new WeakRef(details));
  }

  /** @param {Event} event @param {HTMLElement | null} target @returns {void} */
  #pickTarget(event, target) {
    const process = this.#closestProcess(target);
    const zoom = this.#closestZoom(target);
    this.#pickProcess(event, process);
    this.#pickZoom(zoom);
    this.#pickGraph(event, this.#closestGraph(target));
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestGraph(target) {
    return target?.closest(ViewContinuity.#TAB_SELECTOR) ?? null;
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestProcess(target) {
    return target?.closest('.process-pick') ?? null;
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestZoom(target) {
    return target?.closest('[data-zoom]') ?? null;
  }

  /** @param {Event} event @param {Element | null} node @returns {void} */
  #pickProcess(event, node) {
    if (!node) {
      return;
    }
    this.#processPick(event, /** @type {HTMLElement} */ (node));
  }

  /** @param {Event} event @param {HTMLElement} element @returns {void} */
  #processPick(event, element) {
    const selection = this.#selection();
    const identifier = this.#identifier(element.dataset.group);
    selection.pick?.(event, identifier, Number(element.dataset.pid));
  }

  /** @param {Element | null} node @returns {void} */
  #pickZoom(node) {
    if (!node) {
      return;
    }
    this.#zoomPick(/** @type {HTMLElement} */ (node));
  }

  /** @param {HTMLElement} element @returns {void} */
  #zoomPick(element) {
    const action = this.#zoomAction(element.dataset.zoom);
    const identifier = this.#identifier(element.dataset.group);
    GraphRenderer.zoom(this.#view, identifier, action);
    this.#resetFitOffset(identifier, action);
    GraphRenderer.draw(this.#view, this.#selection(), true);
    this.#restoreOffsets();
  }

  /** @param {string} identifier @param {string} action @returns {void} */
  #resetFitOffset(identifier, action) {
    if (action !== ViewContinuity.#FIT_ACTION) {
      return;
    }
    this.#offsets.set(ViewContinuity.#GROUP_PREFIX + identifier, {top: 0, left: 0});
  }

  /** @param {Event} event @param {Element | null} tab @returns {void} */
  #pickGraph(event, tab) {
    if (!tab) {
      return;
    }
    this.#showGraph(event, /** @type {HTMLElement} */ (tab));
  }

  /** @param {Event} event @param {HTMLElement} tab @returns {void} */
  #showGraph(event, tab) {
    tab.scrollIntoView({block: ViewContinuity.#NEAREST_SCROLL, inline: ViewContinuity.#NEAREST_SCROLL});
    this.#selection().showGraph?.(event, this.#identifier(tab.dataset.graphGroup));
  }

  /** @param {Event} event @returns {void} */
  #rememberFocus(event) {
    const tab = this.#closestGraph(this.#target(event));
    this.#focusedGraph = this.#focusedIdentifier(/** @type {HTMLElement | null} */ (tab));
  }

  /** @param {HTMLElement | null} tab @returns {string} */
  #focusedIdentifier(tab) {
    return tab?.dataset.graphGroup ?? ViewContinuity.#EMPTY_IDENTIFIER;
  }

  /** @returns {void} */
  #restoreGraphFocus() {
    const lost = [document.activeElement === document.body, document.activeElement === null].some(Boolean);
    const tab = this.#elements(ViewContinuity.#TAB_SELECTOR).find(
      /** @param {HTMLElement} element @returns {boolean} */ element => element.dataset.graphGroup === this.#focusedGraph
    );
    this.#focusWhenLost(tab, [lost, tab !== undefined].every(Boolean));
  }

  /** @param {HTMLElement | undefined} tab @param {boolean} lost @returns {void} */
  #focusWhenLost(tab, lost) {
    if (!lost) {
      return;
    }
    /** @type {HTMLElement} */ (tab).focus({preventScroll: true});
  }

  /** @param {KeyboardEvent} event @returns {void} */
  #navigate(event) {
    const target = this.#target(event);
    const tab = target?.closest(ViewContinuity.#TAB_SELECTOR);
    const allowed = [tab !== null, tab !== undefined, ViewContinuity.#NAVIGATION_KEYS.includes(event.key)].every(Boolean);
    this.#navigateWhenAllowed(event, /** @type {HTMLElement | null} */ (tab), allowed);
  }

  /** @param {KeyboardEvent} event @param {HTMLElement | null} tab @param {boolean} allowed @returns {void} */
  #navigateWhenAllowed(event, tab, allowed) {
    if (!allowed) {
      return;
    }
    this.#moveTab(event, /** @type {HTMLElement} */ (tab));
  }

  /** @param {KeyboardEvent} event @param {HTMLElement} tab @returns {void} */
  #moveTab(event, tab) {
    const tabs = this.#elements(ViewContinuity.#TAB_SELECTOR);
    const index = this.#tabIndex(event.key, tabs.indexOf(tab), tabs.length);
    event.preventDefault();
    tabs[index].focus({preventScroll: true});
    this.#showGraph(event, tabs[index]);
  }

  /** @param {string} key @param {number} current @param {number} count @returns {number} */
  #tabIndex(key, current, count) {
    switch (key) {
      case ViewContinuity.#FIRST_KEY:
        return 0;
      case ViewContinuity.#LAST_KEY:
        return count - 1;
      case ViewContinuity.#PREVIOUS_KEY:
        return (current + count - 1) % count;
      default:
        return (current + 1) % count;
    }
  }

  /** @param {string | undefined} action @returns {string} */
  #zoomAction(action) {
    return action ?? ViewContinuity.#FIT_ACTION;
  }
}
