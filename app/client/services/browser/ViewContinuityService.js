@Service
export default class ViewContinuityService {
  /** @type {string[]} */
  static _SCROLL_SELECTORS = ['.logical-scroll', '.history-scroll', '.activity-scroll', '.process-tree-view'];
  /** @type {string} */
  static _GROUP_PREFIX = 'graph-group:';
  /** @type {string} */
  static _AGENT_PREFIX = 'agent:';
  /** @type {string} */
  static _COMMAND_PREFIX = 'command:';
  /** @type {string} */
  static _COMMAND_SELECTOR = '.command-descendants';
  /** @type {string} */
  static _BINDING_ATTRIBUTE = 'binding';
  /** @type {string} */
  static _GRAPH_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static _EXECUTION_SELECTOR = '.execution-history';
  /** @type {string} */
  static _ACTION_SELECTOR = '.action-history-list';
  /** @type {string} */
  static _SUMMARY_SELECTOR = 'summary';
  /** @type {string} */
  static _DETAIL_SELECTOR = '.process-details';
  /** @type {string} */
  static _LIMITATIONS_SELECTOR = '.limitations';
  /** @type {string} */
  static _CLICK_EVENT = 'click';
  /** @type {string} */
  static _KEY_EVENT = 'keydown';
  /** @type {string} */
  static _FOCUS_EVENT = 'focusin';
  /** @type {string} */
  static _TAB_SELECTOR = '[data-agent-tab]';
  /** @type {string} */
  static _AGENT_FOCUS_SELECTOR = '[data-agent-focus]';
  /** @type {string} */
  static _ESCAPE_KEY = 'Escape';
  /** @type {string} */
  static _PROCESS_FOCUS_SELECTOR = '.session-process-pick';
  /** @type {string} */
  static _ZOOM_SELECTOR = '[data-zoom]';
  /** @type {'process'} */
  static _PROCESS_FOCUS_KIND = 'process';
  /** @type {'zoom'} */
  static _ZOOM_FOCUS_KIND = 'zoom';
  /** @type {'agent'} */
  static _AGENT_FOCUS_KIND = 'agent';
  /** @type {string[]} */
  static _HIDDEN_VISIBILITIES = ['hidden', 'collapse'];
  /** @type {string} */
  static _SCROLL_EVENT = 'scroll';
  /** @type {string} */
  static _TOGGLE_EVENT = 'toggle';
  /** @type {string} */
  static _ABORT_EVENT = 'abort';
  /** @type {string} */
  static _EMPTY_IDENTIFIER = '';
  /** @type {string} */
  static _FIT_ACTION = 'fit';
  /** @type {ScrollLogicalPosition} */
  static _NEAREST_SCROLL = 'nearest';
  /** @type {number} */
  static _NO_VISIBLE_HEIGHT = 0;
  /** @type {number} */
  static _RESTORE_MILLISECONDS = 0;
  /** @type {string} */
  static _PREVIOUS_KEY = 'ArrowLeft';
  /** @type {string} */
  static _NEXT_KEY = 'ArrowRight';
  /** @type {string} */
  static _FIRST_KEY = 'Home';
  /** @type {string} */
  static _LAST_KEY = 'End';
  /** @type {string[]} */
  static _NAVIGATION_KEYS = [ViewContinuityService._PREVIOUS_KEY, ViewContinuityService._NEXT_KEY, ViewContinuityService._FIRST_KEY, ViewContinuityService._LAST_KEY];
  /** @type {import('/services/browser/GraphService.js').default | undefined} */
  @Needs
  graph;
  /** @type {import('/services/browser/ChartService.js').default | undefined} */
  @Needs
  chart;
  /** @type {HTMLElement | null} */
  #view = null;
  /** @type {AbortSignal | null} */
  #lifecycle = null;
  /** @type {(() => import('/models/ViewSelection.ts').ViewSelection) | null} */
  #selection = null;
  /** @type {Map<string, {top: number, left: number}>} */
  #offsets = new Map();
  /** @type {Map<string, boolean>} */
  #expanded = new Map();
  /** @type {Map<string, WeakRef<HTMLDetailsElement>>} */
  #toggleElements = new Map();
  /** @type {ReturnType<typeof setTimeout> | null} */
  #timer = null;
  /** @type {string} */
  #focusedTab = ViewContinuityService._EMPTY_IDENTIFIER;
  /** @type {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #focusedControl = null;
  /** @type {(event: Event) => void} */
  #scroll = /** @param {Event} event @returns {void} */ event => this.#rememberScroll(event);
  /** @type {(event: Event) => void} */
  #toggle = /** @param {Event} event @returns {void} */ event => this.#rememberToggle(event);
  /** @type {(event: Event) => void} */
  #click = /** @param {Event} event @returns {void} */ event => this.#pick(event);
  /** @type {(event: Event) => void} */
  #key = /** @param {Event} event @returns {void} */ event => this.#keyboard(/** @type {KeyboardEvent} */ (event));
  /** @type {(event: Event) => void} */
  #focus = /** @param {Event} event @returns {void} */ event => this.#rememberFocus(event);
  /** @type {ResizeObserver} */
  #resize = new ResizeObserver(/** @returns {void} */ () => this.#schedule());
  /** @type {() => void} */
  #abort = /** @returns {void} */ () => this.dispose();
  /** @type {MutationObserver} */
  #observer = new MutationObserver(/** @returns {void} */ () => this.#schedule());

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @param {() => import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  bind(view, lifecycle, selection) {
    this.#disposeBinding();
    this.#requireProviders();
    this.#bindWhenActive(view, lifecycle, selection);
  }

  /** @returns {void} */
  @OnStop
  dispose() {
    this.#disposeBinding();
  }

  /** @param {string} identifier @param {string} selector @returns {void} */
  focusAgent(identifier, selector) {
    this.#focusElement(this.#agentElement(identifier, selector));
  }

  /** @param {string} identifier @param {string} selector @returns {HTMLElement | undefined} */
  #agentElement(identifier, selector) {
    return this.#elements(selector).find(
      /** @param {HTMLElement} element @returns {boolean} */ element => element.dataset.agent === identifier
    );
  }

  /** @param {HTMLElement | undefined} element @returns {void} */
  #focusElement(element) {
    if (element === undefined) {
      return;
    }
    element.focus({preventScroll: true});
  }

  /** @returns {void} */
  #requireProviders() {
    this.#graphProvider();
    this.#chartProvider();
  }

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @param {() => import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #bindWhenActive(view, lifecycle, selection) {
    if (lifecycle.aborted) {
      return;
    }
    this.#view = view;
    this.#lifecycle = lifecycle;
    this.#selection = selection;
    this.#observeView(view, lifecycle);
  }

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @returns {void} */
  #observeView(view, lifecycle) {
    view.addEventListener(ViewContinuityService._CLICK_EVENT, this.#click);
    view.addEventListener(ViewContinuityService._KEY_EVENT, this.#key);
    view.addEventListener(ViewContinuityService._FOCUS_EVENT, this.#focus);
    this.#resize.observe(view);
    view.addEventListener(ViewContinuityService._SCROLL_EVENT, this.#scroll, {capture: true, passive: true});
    view.addEventListener(ViewContinuityService._TOGGLE_EVENT, this.#toggle, true);
    this.#observer.observe(view, {childList: true, subtree: true});
    lifecycle.addEventListener(ViewContinuityService._ABORT_EVENT, this.#abort, {once: true});
  }

  /** @returns {void} */
  #disposeBinding() {
    if (this.#view === null) {
      return;
    }
    this.#releaseView(this.#view, /** @type {AbortSignal} */ (this.#lifecycle));
  }

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @returns {void} */
  #releaseView(view, lifecycle) {
    this.#observer.disconnect();
    this.#resize.disconnect();
    view.removeEventListener(ViewContinuityService._KEY_EVENT, this.#key);
    view.removeEventListener(ViewContinuityService._FOCUS_EVENT, this.#focus);
    this.#cancelRestore();
    view.removeEventListener(ViewContinuityService._SCROLL_EVENT, this.#scroll, true);
    view.removeEventListener(ViewContinuityService._TOGGLE_EVENT, this.#toggle, true);
    view.removeEventListener(ViewContinuityService._CLICK_EVENT, this.#click);
    lifecycle.removeEventListener(ViewContinuityService._ABORT_EVENT, this.#abort);
    this.#clearBinding();
  }

  /** @returns {void} */
  #clearBinding() {
    this.#offsets.clear();
    this.#expanded.clear();
    this.#toggleElements.clear();
    this.#focusedTab = ViewContinuityService._EMPTY_IDENTIFIER;
    this.#focusedControl = null;
    this.#view = null;
    this.#lifecycle = null;
    this.#selection = null;
  }

  /** @returns {HTMLElement} */
  #boundView() {
    if (this.#view === null) {
      throw new Error('The continuity view is not bound.');
    }
    return this.#view;
  }

  /** @returns {import('/models/ViewSelection.ts').ViewSelection} */
  #currentSelection() {
    if (this.#selection === null) {
      throw new Error('The continuity selection is not bound.');
    }
    return this.#selection();
  }

  /** @returns {import('/services/browser/GraphService.js').default} */
  #graphProvider() {
    if (this.graph === undefined) {
      throw new Error('Required browser graph provider is unavailable.');
    }
    return this.graph;
  }

  /** @returns {import('/services/browser/ChartService.js').default} */
  #chartProvider() {
    if (this.chart === undefined) {
      throw new Error('Required browser chart provider is unavailable.');
    }
    return this.chart;
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
    if ([this.#timer !== null, this.#view === null].some(Boolean)) {
      return;
    }
    this.#timer = setTimeout(/** @returns {void} */ () => this.#restore(), ViewContinuityService._RESTORE_MILLISECONDS);
  }

  /** @returns {void} */
  #restore() {
    this.#timer = null;
    this.#restoreExpanded();
    const selection = this.#currentSelection();
    this.#runtimeSelection(selection);
    this.#graphProvider().draw(this.#boundView(), selection);
    this.#restoreTabFocus();
    this.#restoreControlFocus();
    this.#chartProvider().draw(this.#boundView(), selection);
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
    if (!element.matches(ViewContinuityService._ACTION_SELECTOR)) {
      return;
    }
    this.#storeActionScroll(element);
  }

  /** @param {HTMLElement} element @returns {void} */
  #storeActionScroll(element) {
    const identifier = CSS.escape(this.#identifier(element.dataset.session));
    const selector = ViewContinuityService._ACTION_SELECTOR + '[data-session="' + identifier + '"]';
    this.#offsets.set(selector, {top: element.scrollTop, left: element.scrollLeft});
  }

  /** @param {HTMLElement} element @returns {void} */
  #saveGroupScroll(element) {
    if (!element.matches(ViewContinuityService._GRAPH_SELECTOR)) {
      return;
    }
    this.#offsets.set(ViewContinuityService._GROUP_PREFIX + element.dataset.group, {top: element.scrollTop, left: element.scrollLeft});
  }

  /** @param {HTMLElement} element @returns {void} */
  #savePanelScroll(element) {
    for (const selector of ViewContinuityService._SCROLL_SELECTORS) {
      this.#saveMatchingScroll(element, selector);
    }
  }

  /** @param {HTMLElement} element @param {string} selector @returns {void} */
  #saveMatchingScroll(element, selector) {
    const overflow = [element.scrollHeight > element.clientHeight, element.scrollWidth > element.clientWidth].some(Boolean);
    const visibleOverflow = [element.matches(selector), element.clientHeight > ViewContinuityService._NO_VISIBLE_HEIGHT, overflow].every(Boolean);
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
    this.#saveMatchingToggle(element, ViewContinuityService._EXECUTION_SELECTOR, ViewContinuityService._AGENT_PREFIX + element.dataset.agent);
    this.#saveMatchingToggle(element, ViewContinuityService._COMMAND_SELECTOR, ViewContinuityService._COMMAND_PREFIX + element.dataset.binding);
    this.#saveMatchingToggle(element, ViewContinuityService._DETAIL_SELECTOR, ViewContinuityService._DETAIL_SELECTOR);
    this.#saveMatchingToggle(element, ViewContinuityService._LIMITATIONS_SELECTOR, ViewContinuityService._LIMITATIONS_SELECTOR);
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
    if (key.startsWith(ViewContinuityService._GROUP_PREFIX)) {
      return this.#findGroup(key.slice(ViewContinuityService._GROUP_PREFIX.length));
    }
    return this.#queryScroll(key);
  }

  /** @param {string} key @returns {HTMLElement | null} */
  #queryScroll(key) {
    return this.#boundView().querySelector(key);
  }

  /** @param {string} identifier @returns {HTMLElement | undefined} */
  #findGroup(identifier) {
    return this.#elements(ViewContinuityService._GRAPH_SELECTOR).find(
      /** @param {HTMLElement} element @returns {boolean} */
      element => element.dataset.group === identifier
    );
  }

  /** @param {string} selector @returns {HTMLElement[]} */
  #elements(selector) {
    return Array.from(this.#boundView().querySelectorAll(selector),
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
    const agent = this.#findPrefixedToggle(key, ViewContinuityService._AGENT_PREFIX, ViewContinuityService._EXECUTION_SELECTOR, 'agent');
    const command = this.#findPrefixedToggle(key, ViewContinuityService._COMMAND_PREFIX, ViewContinuityService._COMMAND_SELECTOR, ViewContinuityService._BINDING_ATTRIBUTE);
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
    return this.#queryToggle(key);
  }

  /** @param {string} key @returns {HTMLDetailsElement | null} */
  #queryToggle(key) {
    return this.#boundView().querySelector(key);
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
    const element = this.#boundView().querySelector('.runtime-selector select');
    const identifier = this.#identifier(selection.model?.selected);
    this.#setRuntime(/** @type {HTMLSelectElement | null} */ (element), identifier);
  }

  /** @param {string | undefined} identifier @returns {string} */
  #identifier(identifier) {
    return identifier ?? ViewContinuityService._EMPTY_IDENTIFIER;
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
    this.#saveSummaryToggle(element, ViewContinuityService._EXECUTION_SELECTOR, ViewContinuityService._AGENT_PREFIX + element?.dataset.agent, cancelled);
    this.#saveSummaryToggle(element, ViewContinuityService._COMMAND_SELECTOR, ViewContinuityService._COMMAND_PREFIX + element?.dataset.binding, cancelled);
    this.#saveSummaryToggle(element, ViewContinuityService._DETAIL_SELECTOR, ViewContinuityService._DETAIL_SELECTOR, cancelled);
    this.#saveSummaryToggle(element, ViewContinuityService._LIMITATIONS_SELECTOR, ViewContinuityService._LIMITATIONS_SELECTOR, cancelled);
  }

  /** @param {HTMLElement | null} target @returns {HTMLDetailsElement | null} */
  #summaryParent(target) {
    return /** @type {HTMLDetailsElement | null} */ (target?.closest(ViewContinuityService._SUMMARY_SELECTOR)?.parentElement ?? null);
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
    this.#pickTab(event, this.#closestTab(target));
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestTab(target) {
    return target?.closest(ViewContinuityService._TAB_SELECTOR) ?? null;
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestProcess(target) {
    return target?.closest('.process-pick') ?? null;
  }

  /** @param {HTMLElement | null} target @returns {Element | null} */
  #closestZoom(target) {
    return target?.closest(ViewContinuityService._ZOOM_SELECTOR) ?? null;
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
    const selection = this.#currentSelection();
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
    this.#graphProvider().zoom(this.#boundView(), identifier, action);
    this.#resetFitOffset(identifier, action);
    this.#graphProvider().draw(this.#boundView(), this.#currentSelection(), true);
    this.#restoreOffsets();
  }

  /** @param {string} identifier @param {string} action @returns {void} */
  #resetFitOffset(identifier, action) {
    if (action !== ViewContinuityService._FIT_ACTION) {
      return;
    }
    this.#offsets.set(ViewContinuityService._GROUP_PREFIX + identifier, {top: 0, left: 0});
  }

  /** @param {Event} event @param {Element | null} tab @returns {void} */
  #pickTab(event, tab) {
    if (!tab) {
      return;
    }
    this.#showTab(event, /** @type {HTMLElement} */ (tab));
  }

  /** @param {Event} event @param {HTMLElement} tab @returns {void} */
  #showTab(event, tab) {
    tab.scrollIntoView({block: ViewContinuityService._NEAREST_SCROLL, inline: ViewContinuityService._NEAREST_SCROLL});
    this.#currentSelection().showTab?.(event, this.#identifier(tab.dataset.agentTab));
  }

  /** @param {Event} event @returns {void} */
  #rememberFocus(event) {
    const target = this.#target(event);
    const tab = this.#closestTab(target);
    this.#focusedTab = this.#focusedIdentifier(/** @type {HTMLElement | null} */ (tab));
    this.#focusedControl = this.#controlFocus(target);
  }

  /** @param {HTMLElement | null} target @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #controlFocus(target) {
    const process = /** @type {HTMLElement | null | undefined} */ (target?.closest(ViewContinuityService._PROCESS_FOCUS_SELECTOR));
    const zoom = /** @type {HTMLElement | null} */ (this.#closestZoom(target));
    const agent = /** @type {HTMLElement | null | undefined} */ (target?.closest(ViewContinuityService._AGENT_FOCUS_SELECTOR));
    const processFocus = this.#processFocus(process?.dataset);
    const zoomFocus = this.#zoomFocus(zoom?.dataset);
    const control = this.#firstControlFocus(processFocus, zoomFocus);
    return this.#firstControlFocus(control, this.#agentFocus(agent?.dataset));
  }

  /** @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} first @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} second @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #firstControlFocus(first, second) {
    return first ?? second;
  }

  /** @param {DOMStringMap | undefined} data @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #agentFocus(data) {
    if (!data?.agentFocus) {
      return null;
    }
    return {kind: ViewContinuityService._AGENT_FOCUS_KIND, control: data.agentFocus};
  }

  /** @param {DOMStringMap | undefined} data @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #processFocus(data) {
    const known = [data?.session, data?.pid, data?.started, data?.processView].every(Boolean);
    return this.#knownProcessFocus(data, known);
  }

  /** @param {DOMStringMap | undefined} data @param {boolean} known @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #knownProcessFocus(data, known) {
    if (!known) {
      return null;
    }
    const fields = /** @type {DOMStringMap} */ (data);
    return {kind: ViewContinuityService._PROCESS_FOCUS_KIND, session: /** @type {string} */ (fields.session),
      pid: /** @type {string} */ (fields.pid), started: /** @type {string} */ (fields.started), view: /** @type {string} */ (fields.processView)};
  }

  /** @param {DOMStringMap | undefined} data @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #zoomFocus(data) {
    const known = [data?.group, data?.zoom].every(Boolean);
    return this.#knownZoomFocus(data, known);
  }

  /** @param {DOMStringMap | undefined} data @param {boolean} known @returns {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} */
  #knownZoomFocus(data, known) {
    if (!known) {
      return null;
    }
    const fields = /** @type {DOMStringMap} */ (data);
    return {kind: ViewContinuityService._ZOOM_FOCUS_KIND, group: /** @type {string} */ (fields.group), action: /** @type {string} */ (fields.zoom)};
  }

  /** @returns {void} */
  #restoreControlFocus() {
    const element = this.#findControl(this.#focusedControl);
    const lost = this.#focusIsLost();
    this.#focusWhenLost(element, [lost, element !== undefined].every(Boolean));
  }

  /** @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget | null} target @returns {HTMLElement | undefined} */
  #findControl(target) {
    if (target === null) {
      return undefined;
    }
    return this.#matchingControls(target).find(
      /** @param {HTMLElement} element @returns {boolean} */ element => this.#visibleFocus(element)
    );
  }

  /** @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget} target @returns {HTMLElement[]} */
  #matchingControls(target) {
    return this.#elements(this.#focusSelector(target)).filter(
      /** @param {HTMLElement} element @returns {boolean} */ element => this.#matchesFocus(element, target)
    );
  }

  /** @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget} target @returns {string} */
  #focusSelector(target) {
    switch (target.kind) {
      case ViewContinuityService._PROCESS_FOCUS_KIND: return ViewContinuityService._PROCESS_FOCUS_SELECTOR;
      case ViewContinuityService._AGENT_FOCUS_KIND: return ViewContinuityService._AGENT_FOCUS_SELECTOR;
      default: return ViewContinuityService._ZOOM_SELECTOR;
    }
  }

  /** @param {HTMLElement} element @param {import('/models/BrowserFocusTarget.ts').BrowserFocusTarget} target @returns {boolean} */
  #matchesFocus(element, target) {
    switch (target.kind) {
      case ViewContinuityService._PROCESS_FOCUS_KIND: return [element.dataset.session === target.session, element.dataset.pid === target.pid,
        element.dataset.started === target.started, element.dataset.processView === target.view].every(Boolean);
      case ViewContinuityService._AGENT_FOCUS_KIND: return element.dataset.agentFocus === target.control;
      default: return [element.dataset.group === target.group, element.dataset.zoom === target.action].every(Boolean);
    }
  }

  /** @param {HTMLElement} element @returns {boolean} */
  #visibleFocus(element) {
    const visibility = getComputedStyle(element).visibility;
    return [element.isConnected, element.getClientRects().length > 0,
      !ViewContinuityService._HIDDEN_VISIBILITIES.includes(visibility)].every(Boolean);
  }

  /** @returns {boolean} */
  #focusIsLost() {
    return [document.activeElement === document.body, document.activeElement === null].some(Boolean);
  }

  /** @param {HTMLElement | null} tab @returns {string} */
  #focusedIdentifier(tab) {
    return tab?.dataset.agentTab ?? ViewContinuityService._EMPTY_IDENTIFIER;
  }

  /** @returns {void} */
  #restoreTabFocus() {
    const lost = this.#focusIsLost();
    const tab = this.#elements(ViewContinuityService._TAB_SELECTOR).find(
      /** @param {HTMLElement} element @returns {boolean} */ element => element.dataset.agentTab === this.#focusedTab
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
  #keyboard(event) {
    this.#navigate(event);
    this.#escape(event);
  }

  /** @param {KeyboardEvent} event @returns {void} */
  #escape(event) {
    if (event.key !== ViewContinuityService._ESCAPE_KEY) {
      return;
    }
    this.#currentSelection().close?.(event);
  }

  /** @param {KeyboardEvent} event @returns {void} */
  #navigate(event) {
    const target = this.#target(event);
    const tab = target?.closest(ViewContinuityService._TAB_SELECTOR);
    const allowed = [tab !== null, tab !== undefined, ViewContinuityService._NAVIGATION_KEYS.includes(event.key)].every(Boolean);
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
    const tabs = this.#elements(ViewContinuityService._TAB_SELECTOR);
    const index = this.#tabIndex(event.key, tabs.indexOf(tab), tabs.length);
    event.preventDefault();
    tabs[index].focus({preventScroll: true});
    this.#showTab(event, tabs[index]);
  }

  /** @param {string} key @param {number} current @param {number} count @returns {number} */
  #tabIndex(key, current, count) {
    switch (key) {
      case ViewContinuityService._FIRST_KEY:
        return 0;
      case ViewContinuityService._LAST_KEY:
        return count - 1;
      case ViewContinuityService._PREVIOUS_KEY:
        return (current + count - 1) % count;
      default:
        return (current + 1) % count;
    }
  }

  /** @param {string | undefined} action @returns {string} */
  #zoomAction(action) {
    return action ?? ViewContinuityService._FIT_ACTION;
  }
}
