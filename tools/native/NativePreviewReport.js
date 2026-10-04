class NativePreviewReport {
  /** @type {string} */
  static #READY_SELECTOR = '.logical-panel';
  /** @type {string} */
  static #TAB_SELECTOR = '.session-tab';
  /** @type {string} */
  static #CLOSE_SELECTOR = '.session-panel-close';
  /** @type {string} */
  static #OVERVIEW = "What's running";
  /** @type {string} */
  static #RESOURCES = 'Resources';
  /** @type {string} */
  static #TREE_VIEW_SELECTOR = '.process-tree-view';
  /** @type {string} */
  static #RUNNING = 'running';
  /** @type {string} */
  static #TABLE = 'table';
  /** @type {string} */
  static #TREE = 'tree';
  /** @type {string} */
  static #HISTORY = 'history';
  /** @type {string} */
  static #POLLS = 'polls';
  /** @type {string} */
  static #HOLD = 'hold';
  /** @type {string} */
  static #PROCESSES = 'Process tree';
  /** @type {string} */
  static #GRAPH_TAB_SELECTOR = '[role=tab][data-session-tab]';
  /** @type {string} */
  static #COORDINATOR_SOURCE = 'Coordinator-reported team roster';

  /** @type {string} */
  static #STATUS_CONTROL_SELECTOR = '.status-controls w-btn';
  /** @type {string} */
  static #LAYOUT = 'layout';
  /** @type {string} */
  static #ASSERT_READY = 'assert-ready';
  /** @type {string} */
  static #ZOOM = 'zoom';
  /** @type {string} */
  static #EMPTY_TEXT = '';
  /** @type {string} */
  static #SELECTED_TAB_SELECTOR = '.session-panel.is-open .session-tab[aria-selected="true"]';
  /** @type {string} */
  static #UPDATES = 'updates';
  /** @type {string} */
  static #PAUSED = 'paused';
  /** @type {string} */
  static #GRAPH_NODE_SELECTOR = '.graph-node';
  /** @type {string} */
  static #COMMAND_RESOURCE_SELECTOR = '.history-card';
  /** @type {string} */
  static #ACTION_RUNNING = 'action-running';
  /** @type {string} */
  static #ACTION_ROW_SELECTOR = '.action-history-row';
  /** @type {string} */
  static #ACTION_NAME_SELECTOR = '.action-history-heading strong';
  /** @type {string} */
  static #ELAPSED_SELECTOR = '.action-elapsed';
  /** @type {string} */
  static #AGENT_ROW_SELECTOR = '.agent-row';
  /** @type {string} */
  static #AGENT_ATTRIBUTE = 'data-agent';
  /** @type {string} */
  static #SESSION_CONTROL_SELECTOR = '.session-card-open';
  /** @type {string} */
  static #PANEL_SELECTOR = '#session-panel';
  /** @type {number} */
  static #EMPTY_COUNT = 0;
  /** @type {number} */
  static #SINGLE_COLUMN = 1;
  /** @type {number} */
  static #MINIMUM_TAB_HEIGHT = 36;

  /** @type {number} */
  static #WAIT_MILLISECONDS = 20000;
  /** @type {number} */
  static #POLL_MILLISECONDS = 80;
  /** @type {number} */
  static #HOLD_MILLISECONDS = 6500;
  /** @type {number} */
  static #REQUIRED_POLLS = 3;
  /** @type {number} */
  static #REQUIRED_GROUPS = 2;
  /** @type {number} */
  static #MAXIMUM_COLUMNS = 4;
  /** @type {number} */
  static #SCROLL_DISTANCE = 120;
  /** @type {number} */
  static #MAXIMUM_ZOOM = 1.5;
  /** @type {number} */
  static #SIZE_TOLERANCE = 1;
  /** @type {number} */
  static #FIRST_SAMPLE = 1;
  /** @type {string} */
  static #SAMPLE_SELECTOR = '.main-footer > span:last-child';
  /** @type {string} */
  static #GROUP_SELECTOR = '.graph-group';
  /** @type {string} */
  static #SCROLL_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static #TABLE_GRID = '.session-process-grid';
  /** @type {string} */
  static #TABLE_CARD = '.session-process-card';
  /** @type {string} */
  static #HISTORY_GRID = '.history-grid';
  /** @type {string} */
  static #HISTORY_CARD = '.history-card';
  /** @type {string} */
  static #STALE = 'stale';

  /** @param {{label: string, action: string, section: string, mode: string, width: number, height: number, check: string, actionName?: string, agentRecordId?: string, sessionId?: string, taskId?: string, actionRecordId?: string}} input @param {boolean} iconLoaded @returns {Promise<string>} */
  static async verify(input, iconLoaded) {
    this.#rememberActionName(input.actionName);
    this.#rememberOwner(input);
    this.#requireFocusedOwner(input);
    document.body.dataset.nativeQaStage = "navigation";
    await this.#navigation();
    document.body.dataset.nativeQaStage = "baseline";
    await this.#sampleBaseline();
    const before = this.#samples();
    document.body.dataset.nativeQaBefore = String(before);
    document.body.dataset.nativeQaStage = "action";
    await this.#perform(input.action);
    document.body.dataset.nativeQaStage = "settle";
    await this.#settle(input, before);
    document.body.dataset.nativeQaStage = "finish";
    await this.#finishCondition(input);
    await this.#captureInteraction(input.check);
    const report = this.#report(iconLoaded);
    this.#assertReport(report, input);
    this.#assertNodeLabels(input.check);
    return JSON.stringify({...report, actionHistory: this.#actionRecords(), verification: {success: true, label: input.label, requested: input}});
  }

  /** @param {string | undefined} name @returns {void} */
  static #rememberActionName(name) {
    document.body.dataset.qaActionName = name ?? 'offline_ui_validation';
  }

  /** @param {{agentRecordId?: string, sessionId?: string, taskId?: string, actionRecordId?: string}} input @returns {void} */
  static #rememberOwner(input) {
    document.body.dataset.qaAgentId = input.agentRecordId ?? this.#EMPTY_TEXT;
    document.body.dataset.qaSessionId = input.sessionId ?? this.#EMPTY_TEXT;
    document.body.dataset.qaTaskId = input.taskId ?? this.#EMPTY_TEXT;
    document.body.dataset.qaExpectedActionId = input.actionRecordId ?? this.#EMPTY_TEXT;
  }

  /** @param {{check: string, agentRecordId?: string, sessionId?: string, taskId?: string, actionRecordId?: string}} input @returns {void} */
  static #requireFocusedOwner(input) {
    const focused = ['current-work-live', 'current-work-hidden', 'current-work-completed', 'current-task-only', 'current-task-polls', this.#ACTION_RUNNING, 'action-progress', 'action-history'].includes(input.check);
    const supplied = [input.agentRecordId, input.sessionId, input.taskId, input.actionRecordId].every(/** @param {string | undefined} field @returns {boolean} */ field => Boolean(field?.trim()));
    if ([focused, !supplied].every(Boolean)) throw new Error('Native QA identity requires explicit agent, session, task and action record IDs.');
  }

  // Clicks the first session card control; the card, not an agent row, opens the session panel.
  /** @returns {Promise<void>} */
  static async #openFirstSession() {
    const started = performance.now();
    while (document.querySelector(this.#SESSION_CONTROL_SELECTOR) === null) await this.#waitIteration(started);
    /** @type {HTMLElement} */ (document.querySelector(this.#SESSION_CONTROL_SELECTOR)).click();
    await this.#waitIteration(started);
  }

  /** @returns {Promise<void>} */
  static async #navigation() {
    const started = performance.now();
    await this.#waitNavigation(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitNavigation(started) {
    while (document.querySelector(this.#READY_SELECTOR) === null) await this.#waitIteration(started);
  }

  /** @returns {Promise<void>} */
  static async #sampleBaseline() {
    const started = performance.now();
    await this.#waitSampleBaseline(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitSampleBaseline(started) {
    while (!Number.isFinite(this.#samples())) await this.#waitIteration(started);
  }

  /** @param {boolean} iconLoaded @returns {string} */
  static read(iconLoaded) {
    return JSON.stringify(this.#report(iconLoaded));
  }

  /** @param {string} action @returns {Promise<void>} */
  static async #perform(action) {
    switch (action) {
      case this.#RUNNING: await this.section(this.#OVERVIEW); return;
      case 'current-expiry': await this.section(this.#OVERVIEW); return;
      case 'descendants': await this.section(this.#OVERVIEW); await this.#openDescendants(); return;
      case 'owned-tree': await this.#ownedTree(); return;
      case 'owned-table': throw this.#removed(action);
      case this.#TABLE: throw this.#removed(action);
      case this.#TREE: await this.processes(); return;
      case 'connector-first': await this.#connectorAt(0); return;
      case 'connector-second': await this.#connectorAt(1); return;
      case 'exercise': await this.exercise(); return;
      case 'pause': this.pause(); return;
      case 'resume': this.resume(); return;
      case this.#HISTORY: await this.section(this.#RESOURCES); return;
      case 'activity': await this.section('Activity'); return;
      case 'runtimes': throw this.#removed(action);
      case this.#POLLS: return;
      case 'settle': return;
      case 'open-first-session': await this.#openFirstSession(); return;
      case 'open-first-agent': await this.#openFirstSession(); return;
      case this.#HOLD: throw new Error('Unsupported native QA action: ' + action);
      default: throw new Error('Unknown native QA action: ' + action);
    }
  }

  /** @param {string} action @returns {Error} */
  static #removed(action) {
    return new Error('Unsupported native QA action: ' + action + '. The Table view and the Runtimes page were removed; use the session details Process tree or Resources tab.');
  }

  /** @returns {string} */
  static #currentSection() {
    const tab = document.querySelector(this.#SELECTED_TAB_SELECTOR);
    if (tab === null) return this.#OVERVIEW;
    return this.#text(tab);
  }

  /** @param {string} name @returns {Promise<void>} */
  static async section(name) {
    if (this.#currentSection() === name) return;
    await this.#showSection(name);
  }

  /** @param {string} name @returns {Promise<void>} */
  static async #showSection(name) {
    if (name === this.#OVERVIEW) {
      this.#press(document.querySelector(this.#CLOSE_SELECTOR), 'session details close');
      return;
    }
    await this.#openDetails();
    this.#click(this.#TAB_SELECTOR, name);
  }

  /** @returns {Promise<void>} */
  static async #openDetails() {
    if (document.querySelector(this.#TAB_SELECTOR) !== null) return;
    this.#press(this.#detailControl(), 'session card');
    await this.#waitTabs(performance.now());
  }

  // The card control of the session holding the owned agent row, else the first session card.
  /** @returns {Element | null} */
  static #detailControl() {
    const rows = /** @type {HTMLElement[]} */ (Array.from(document.querySelectorAll(this.#AGENT_ROW_SELECTOR)));
    const owned = rows.find(/** @param {HTMLElement} row @returns {boolean} */ row => row.dataset.agent === document.body.dataset.qaAgentId);
    const card = owned?.closest('.session-card')?.querySelector(this.#SESSION_CONTROL_SELECTOR) ?? null;
    return card ?? document.querySelector(this.#SESSION_CONTROL_SELECTOR);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitTabs(started) {
    while (document.querySelector(this.#TAB_SELECTOR) === null) await this.#waitIteration(started);
  }

  /** @returns {Promise<void>} */
  static async processes() {
    await this.section(this.#PROCESSES);
  }

  /** @returns {Promise<void>} */
  static async #openDescendants() {
    const started = performance.now();
    await this.#waitChildCards(started);
    this.#press(document.querySelector('.command-descendants summary'), 'linked command descendants');
    await this.#layout();
    this.#scrollChild();
  }

  /** @returns {void} */
  static #scrollChild() {
    document.querySelector('.command-child')?.scrollIntoView({block: 'center', inline: 'nearest'});
  }

  /** @param {string} selector @returns {void} */
  static #scrollOwner(selector) {
    this.#ownerNode(selector)?.scrollIntoView({block: 'center', inline: 'center'});
  }

  /** @param {string} selector @returns {HTMLElement | undefined} */
  static #ownerNode(selector) {
    return /** @type {HTMLElement[]} */ (Array.from(document.querySelectorAll(selector))).find(
      /** @param {HTMLElement} element @returns {boolean} */ element => this.#text(element).includes('Current child lineage'));
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitOwnedNode(started) {
    while (this.#ownerNode('.graph-node') === undefined) await this.#waitIteration(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitChildCards(started) {
    while (document.querySelector('.command-child') === null) await this.#waitIteration(started);
  }

  /** @returns {Promise<void>} */
  static async #ownedTree() {
    await this.processes();
    await this.#waitOwnedNode(performance.now());
    this.#scrollOwner('.graph-node');
  }

  /** @param {number} index @returns {Promise<void>} */
  static async #connectorAt(index) {
    await this.processes();
    await this.#waitConnectorCount(index, performance.now());
    const identifier = this.#groupIds()[index];
    await this.connector(identifier);
  }

  /** @param {number} index @param {number} started @returns {Promise<void>} */
  static async #waitConnectorCount(index, started) {
    while (this.#groupIds().length <= index) await this.#waitIteration(started);
  }

  /** @param {string} identifier @returns {Promise<void>} */
  static async connector(identifier) {
    await this.processes();
    await this.#selectGroup(identifier);
  }

  /** @returns {void} */
  static pause() {
    throw new Error('Unsupported native QA action: pause');
  }

  /** @returns {void} */
  static resume() {
    throw new Error('Unsupported native QA action: resume');
  }

  /** @returns {Promise<void>} */
  static async exercise() {
    await this.#layout();
    const ids = this.#groupIds();
    this.#requireGroups(ids.length);
    const before = await this.#observeGroups(ids);
    const first = await this.#selectGroup(ids[0]);
    this.#zoomFirst(first);
    document.body.dataset.nativeQaStage = this.#LAYOUT;
    await this.#layout();
    document.body.dataset.nativeQaStage = this.#ASSERT_READY;
    const after = await this.#observeGroups(ids);
    await this.#selectGroup(ids[0]);
    const scroll = this.#overflowScroll();
    this.#scroll(scroll);
    document.body.dataset.qaBaseline = JSON.stringify({before, after, scroll: this.#scrollRecord(scroll)});
  }

  /** @returns {string[]} */
  static #groupIds() {
    return this.#groups().slice(0, this.#REQUIRED_GROUPS).map(
      /** @param {Element} group @returns {string} */
      group => String(/** @type {HTMLElement} */ (group).dataset.group)
    );
  }

  /** @param {string} id @returns {Promise<Element>} */
  static async #selectGroup(id) {
    const started = performance.now();
    const group = await this.#waitGroup(id, started);
    group.scrollIntoView({block: 'nearest', inline: 'nearest'});
    document.body.dataset.nativeQaStage = this.#LAYOUT;
    await this.#layout();
    document.body.dataset.nativeQaStage = this.#ASSERT_READY;
    return this.#waitGroup(id, started);
  }

  /** @param {string} id @param {number} started @returns {Promise<Element>} */
  static async #waitGroup(id, started) {
    while (this.#group(id) === undefined) await this.#waitNamedGroup(id, started);
    return /** @type {Element} */ (this.#group(id));
  }

  /** @param {string} id @param {number} started @returns {Promise<void>} */
  static async #waitNamedGroup(id, started) {
    if (performance.now() - started >= this.#WAIT_MILLISECONDS) throw new Error('Native QA timed out waiting for process group ' + id);
    await this.#waitIteration(started);
  }

  /** @param {string} id @returns {Element | undefined} */
  static #group(id) {
    return this.#groups().find(/** @param {Element} group @returns {boolean} */ group => /** @type {HTMLElement} */ (group).dataset.group === id);
  }

  /** @param {string[]} ids @returns {Promise<{id: string, scale: string}[]>} */
  static async #observeGroups(ids) {
    /** @type {{id: string, scale: string}[]} */
    const observed = [];
    for (const id of ids) { const group = await this.#selectGroup(id); observed.push(this.#scale(group)); }
    return observed;
  }

  /** @param {string} check @returns {Promise<void>} */
  static async #captureInteraction(check) {
    const requested = [this.#ZOOM, this.#POLLS].includes(check);
    await this.#captureRequested(requested);
  }

  /** @param {boolean} requested @returns {Promise<void>} */
  static async #captureRequested(requested) {
    if (!requested) {
      return;
    }
    await this.#captureGroups();
  }

  /** @returns {Promise<void>} */
  static async #captureGroups() {
    const baseline = this.#baselineGroups(this.#interaction());
    const ids = baseline.map(/** @param {{id: string, scale: string}} group @returns {string} */ group => group.id);
    this.#requireGroups(ids.length);
    const current = await this.#observeGroups(ids);
    document.body.dataset.qaCurrent = JSON.stringify(current);
    await this.#selectGroup(ids[0]);
  }

  /** @param {{baseline: {after: {id: string, scale: string}[]}} | null} interaction @returns {{id: string, scale: string}[]} */
  static #baselineGroups(interaction) {
    if (interaction === null) {
      throw new Error('Native QA process zoom baseline unavailable.');
    }
    return interaction.baseline.after;
  }

  /** @returns {Element[]} */
  static #groups() {
    return Array.from(document.querySelectorAll(this.#GROUP_SELECTOR)).filter(/** @param {Element} group @returns {boolean} */ group => group.querySelector('.graph-canvas') !== null);
  }

  /** @param {number} count @returns {void} */
  static #requireGroups(count) {
    if (count < this.#REQUIRED_GROUPS) throw new Error('Native QA needs two actual process groups for independent zoom.');
  }

  /** @param {Element} group @returns {void} */
  static #zoomFirst(group) {
    const current = this.#scale(group);
    const level = Number.parseFloat(current.scale.replace('scale(', this.#EMPTY_TEXT));
    const action = this.#zoomAction(level);
    const button = group.querySelector('[data-zoom="' + action + '"]');
    this.#press(button, 'group zoom ' + action);
  }

  /** @param {number} level @returns {string} */
  static #zoomAction(level) {
    if (level >= this.#MAXIMUM_ZOOM) return 'out';
    return 'in';
  }

  /** @param {Element} group @returns {{id: string, scale: string}} */
  static #scale(group) {
    const canvas = /** @type {HTMLElement | null} */ (group.querySelector('.graph-canvas'));
    return {id: String(/** @type {HTMLElement} */ (group).dataset.group), scale: this.#transform(canvas)};
  }

  /** The SVG canvas zooms by scaled width over its unscaled viewBox width. @param {HTMLElement | null} canvas @returns {string} */
  static #transform(canvas) {
    if (canvas === null) return this.#EMPTY_TEXT;
    const base = Number(canvas.getAttribute('viewBox')?.split(' ')[2]);
    return 'scale(' + Math.round(Number(canvas.getAttribute('width')) / base * 1e6) / 1e6 + ')';
  }

  /** @returns {HTMLElement | null} */
  static #overflowScroll() {
    const element = Array.from(document.querySelectorAll(this.#SCROLL_SELECTOR)).find(
      /** @param {Element} item @returns {boolean} */
      item => [item.scrollHeight > item.clientHeight, item.scrollWidth > item.clientWidth].some(Boolean)
    );
    return this.#element(element);
  }

  /** @param {Element | undefined} element @returns {HTMLElement | null} */
  static #element(element) {
    return /** @type {HTMLElement | null} */ (element ?? null);
  }

  /** @param {HTMLElement | null} scroll @returns {void} */
  static #scroll(scroll) {
    if (scroll === null) throw new Error('Native QA needs an overflowing real process tree for pan.');
    scroll.scrollTop = Math.min(this.#SCROLL_DISTANCE, scroll.scrollHeight - scroll.clientHeight);
    scroll.scrollLeft = Math.min(this.#SCROLL_DISTANCE, scroll.scrollWidth - scroll.clientWidth);
    scroll.dispatchEvent(new Event('scroll'));
  }

  /** @param {HTMLElement | null} scroll @returns {{id: string, top: number, left: number} | null} */
  static #scrollRecord(scroll) {
    if (scroll === null) return null;
    return {id: String(scroll.dataset.group), top: scroll.scrollTop, left: scroll.scrollLeft};
  }

  /** @param {string} selector @param {string} text @returns {void} */
  static #click(selector, text) {
    const button = Array.from(document.querySelectorAll(selector)).find(
      /** @param {Element} item @returns {boolean} */
      item => String(item.textContent).includes(text)
    );
    this.#press(button, text);
  }

  /** @param {Element | null | undefined} button @param {string} label @returns {void} */
  static #press(button, label) {
    if (!button) throw new Error('Native QA control is missing: ' + label);
    /** @type {HTMLElement} */ (button).click();
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @returns {Promise<void>} */
  static async #settle(input, before) {
    const started = performance.now();
    await this.#wait(input, before, started);
    document.body.dataset.nativeQaStage = this.#LAYOUT;
    await this.#layout();
    document.body.dataset.nativeQaStage = this.#ASSERT_READY;
    this.#requireReady(this.#ready(input, before), input.section);
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @param {number} started @returns {Promise<void>} */
  static async #wait(input, before, started) {
    while (!this.#ready(input, before)) await this.#waitIteration(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitIteration(started) {
    this.#requireTime(started);
    this.#requireNative();
    /** @type {Promise<void>} */
    const waiting = new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { setTimeout(resolve, this.#POLL_MILLISECONDS); }
    );
    await waiting;
  }

  /** @param {number} started @returns {void} */
  static #requireTime(started) {
    if (performance.now() - started >= this.#WAIT_MILLISECONDS) throw new Error('Native QA timed out waiting for the requested view, dimensions or samples.');
  }

  /** @returns {void} */
  static #requireNative() {
    const failure = document.querySelector('[data-native-failure]');
    if (failure) throw new Error('Native app reported a failure: ' + String(failure.getAttribute('data-native-failure')).slice(0, 300));
  }

  /** @returns {Promise<void>} */
  static #layout() {
    return new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { setTimeout(resolve, this.#POLL_MILLISECONDS); }
    );
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @returns {boolean} */
  static #ready(input, before) {
    const section = this.#currentSection();
    const mode = this.#mode();
    return [section.includes(input.section), this.#modeReady(input.mode, mode),
      this.#viewportReady(input),
      this.#conditionReady(input.check, before)].every(Boolean);
  }

  /** @returns {string} */
  static #mode() {
    if (document.querySelector(this.#TREE_VIEW_SELECTOR) === null) return this.#EMPTY_TEXT;
    return this.#TREE;
  }

  /** @param {{width: number, height: number, check: string}} input @returns {boolean} */
  static #viewportReady(input) {
    if (input.check === this.#UPDATES) return true;
    return [Math.abs(innerWidth - input.width) <= this.#SIZE_TOLERANCE,
      Math.abs(innerHeight - input.height) <= this.#SIZE_TOLERANCE].every(Boolean);
  }

  /** @param {string} expected @param {string} actual @returns {boolean} */
  static #modeReady(expected, actual) {
    if (expected === this.#EMPTY_TEXT) return true;
    return actual === expected;
  }

  /** @param {Element | null} element @returns {string} */
  static #class(element) {
    if (element === null) return this.#EMPTY_TEXT;
    return String(element.className);
  }

  /** @param {string} check @param {number} before @returns {boolean} */
  static #conditionReady(check, before) {
    switch (check) {
      case this.#POLLS: return this.#samples() >= before + this.#REQUIRED_POLLS;
      case this.#UPDATES: return this.#samples() >= before + this.#REQUIRED_POLLS;
      case this.#PAUSED: return this.#live().toLowerCase().includes(this.#PAUSED);
      case 'resumed': return [!this.#live().toLowerCase().includes(this.#PAUSED), this.#samples() > Number(document.body.dataset.qaPausedSamples)].every(Boolean);
      case this.#TREE: return document.querySelectorAll(this.#GRAPH_NODE_SELECTOR).length > this.#EMPTY_COUNT;
      case this.#HISTORY: return document.querySelectorAll(this.#HISTORY_CARD).length > this.#EMPTY_COUNT;
      case 'logical': return this.#logicalReady();
      case 'current-work-live': return this.#currentWorkVisible();
      case 'current-task-only': return this.#currentTaskOnly();
      case 'current-task-polls': return [this.#currentTaskOnly(), this.#samples() >= before + this.#REQUIRED_POLLS].every(Boolean);
      case 'current-work-hidden': return this.#currentWorkHidden();
      case 'current-work-completed': return this.#historyComplete();
      case 'action-history': return this.#historyComplete();
      case 'descendant-open-polls': return [this.#samples() >= before + this.#REQUIRED_POLLS, document.querySelector('.command-descendants')?.hasAttribute('open') === true, document.querySelector('.command-child') !== null].every(Boolean);
      case 'descendant-linked': return document.querySelector('.command-child') !== null;
      case 'descendant-owner': return Array.from(document.querySelectorAll('.process-owner')).some(/** @param {Element} label @returns {boolean} */ label => this.#text(label).includes('Current child lineage'));
      case 'command-linked': return document.querySelector(this.#COMMAND_RESOURCE_SELECTOR) !== null;
      case this.#ACTION_RUNNING: return this.#runningAction() !== undefined;
      case 'action-progress': return [this.#samples() >= before + this.#REQUIRED_POLLS, this.#actionAdvanced()].every(Boolean);
      default: return this.#samples() >= this.#FIRST_SAMPLE;
    }
  }

  /** @returns {boolean} */
  static #logicalReady() {
    const records = this.#logicalRecords();
    return [records.length > this.#EMPTY_COUNT, records.every(this.#freshLogical.bind(this))].every(Boolean);
  }

  /** @returns {HTMLElement | undefined} */
  static #currentWorkRow() {
    return /** @type {HTMLElement[]} */ (Array.from(document.querySelectorAll(this.#AGENT_ROW_SELECTOR))).find(
      /** @param {HTMLElement} row @returns {boolean} */ row => row.dataset.agent === document.body.dataset.qaAgentId);
  }

  /** @param {HTMLElement | undefined} row @returns {boolean} */
  static #currentOwner(row) {
    return [row !== undefined, row?.dataset.session === document.body.dataset.qaSessionId,
      row?.dataset.task === document.body.dataset.qaTaskId,
      row?.dataset.source === this.#COORDINATOR_SOURCE].every(Boolean);
  }

  /** @returns {boolean} */
  static #currentWorkVisible() {
    const row = this.#currentWorkRow();
    const action = this.#text(row?.querySelector('.agent-tool-name') ?? null);
    const status = row?.dataset.status ?? this.#EMPTY_TEXT;
    return [this.#currentOwner(row), ['active', this.#RUNNING].includes(status), action === document.body.dataset.qaActionName].every(Boolean);
  }

  /** @returns {boolean} */
  static #currentTaskOnly() {
    const row = this.#currentWorkRow();
    const status = row?.dataset.status ?? this.#EMPTY_TEXT;
    const task = this.#text(row?.querySelector('.agent-task') ?? null);
    return [this.#currentOwner(row), ['active', this.#RUNNING].includes(status), Boolean(task.trim()),
      row?.querySelector('.agent-tool.is-idle') !== null].every(Boolean);
  }

  /** @returns {boolean} */
  static #currentWorkHidden() {
    return this.#currentWorkRow() === undefined;
  }

  /** @param {{id: string, name: string, status: string, source: string, task: string}} record @returns {boolean} */
  static #freshLogical(record) {
    return [Boolean(record.id), ['active', this.#RUNNING].includes(record.status), Boolean(record.source.trim()),
      Boolean(record.task.trim()), !record.source.toLowerCase().includes('recovered')].every(Boolean);
  }

  /** @returns {boolean} */
  static #historyComplete() {
    const rows = Array.from(document.querySelectorAll(this.#ACTION_ROW_SELECTOR));
    return rows.some(/** @param {Element} row @returns {boolean} */ row => this.#completedHistory(row));
  }

  /** @param {Element} row @returns {boolean} */
  static #actionOwner(row) {
    const data = /** @type {HTMLElement} */ (row).dataset;
    return [data.agent === document.body.dataset.qaAgentId,
      data.session === document.body.dataset.qaSessionId, data.task === document.body.dataset.qaTaskId,
      data.action === document.body.dataset.qaExpectedActionId, data.source === this.#COORDINATOR_SOURCE,
      this.#text(row.querySelector(this.#ACTION_NAME_SELECTOR)) === document.body.dataset.qaActionName].every(Boolean);
  }

  /** @returns {HTMLElement | undefined} */
  static #runningAction() {
    return Array.from(document.querySelectorAll(this.#ACTION_ROW_SELECTOR), /** @param {Element} row @returns {HTMLElement} */ row => /** @type {HTMLElement} */ (row)).find(
      /** @param {HTMLElement} row @returns {boolean} */
      row => [this.#actionOwner(row), row.dataset.status === this.#RUNNING,
        row.dataset.running === 'true', row.querySelector(this.#ELAPSED_SELECTOR) !== null].every(Boolean)
    );
  }

  /** @returns {boolean} */
  static #actionAdvanced() {
    const row = this.#runningAction();
    return [row !== undefined, row?.dataset.action === document.body.dataset.qaActionId,
      this.#text(row?.querySelector(this.#ELAPSED_SELECTOR) ?? null) !== document.body.dataset.qaActionElapsed].every(Boolean);
  }

  /** @returns {{id: string | undefined, agent: string | undefined, sessionId: string | undefined, taskId: string | undefined, source: string | undefined, running: boolean, name: string, status: string, elapsed: string, text: string}[]} */
  static #actionRecords() {
    return Array.from(document.querySelectorAll(this.#ACTION_ROW_SELECTOR), this.#actionRecord.bind(this));
  }

  /** @param {Element} row @returns {{id: string | undefined, agent: string | undefined, sessionId: string | undefined, taskId: string | undefined, source: string | undefined, running: boolean, name: string, status: string, elapsed: string, text: string}} */
  static #actionRecord(row) {
    const data = /** @type {HTMLElement} */ (row).dataset;
    return {id: data.action, agent: data.agent, sessionId: data.session, taskId: data.task,
      source: data.source, running: data.running === 'true',
      name: this.#text(row.querySelector(this.#ACTION_NAME_SELECTOR)), status: data.status ?? this.#EMPTY_TEXT,
      elapsed: this.#text(row.querySelector(this.#ELAPSED_SELECTOR)), text: this.#text(row)};
  }

  /** @param {Element} row @returns {boolean} */
  static #completedHistory(row) {
    const data = /** @type {HTMLElement} */ (row).dataset;
    return [this.#actionOwner(row), data.status === 'completed', data.running === 'false',
      this.#text(row).includes('Measured by the reporting caller')].every(Boolean);
  }

  /** @param {boolean} ready @param {string} section @returns {void} */
  static #requireReady(ready, section) {
    if (!ready) throw new Error('Native QA view changed before capture: ' + section);
  }

  /** @param {{check: string}} input @returns {Promise<void>} */
  static async #finishCondition(input) {
    await this.#holdWhenNeeded(input.check);
    this.#rememberPause(input.check);
    this.#rememberAction(input.check);
    this.#requireNative();
  }

  /** @param {string} check @returns {void} */
  static #rememberAction(check) {
    if (check !== this.#ACTION_RUNNING) return;
    const row = this.#runningAction();
    document.body.dataset.qaActionId = row?.dataset.action;
    document.body.dataset.qaActionElapsed = this.#text(row?.querySelector(this.#ELAPSED_SELECTOR) ?? null);
  }

  /** @param {string} check @returns {Promise<void>} */
  static async #holdWhenNeeded(check) {
    if (check !== this.#HOLD) return;
    /** @type {Promise<void>} */
    const waiting = new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { setTimeout(resolve, this.#HOLD_MILLISECONDS); }
    );
    await waiting;
  }

  /** @param {string} check @returns {void} */
  static #rememberPause(check) {
    if (check !== this.#PAUSED) return;
    document.body.dataset.qaPausedSamples = String(this.#samples());
  }

  /** @param {boolean} iconLoaded @returns {{commandResources: {binding: string | undefined, text: string}[], qaStage: string | undefined, qaBefore: string | undefined, capturedAt: string, native: string | undefined, section: string, samples: string, sampleCount: number, width: number, height: number, horizontalOverflow: boolean, documentScroll: boolean, live: string, groups: number, graphGroups: number, graphNodes: number, graphTabBounds: {id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}[], tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextTableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextHistoryLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, chartPoints: number | null, chartUnit: string | null, sessionContainers: number, flatAgentCards: number, historyCards: number, runtimeCards: number, activityCards: number, sidebarOverflow: boolean, summaryBounds: {text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}[], logicalRecords: {id: string, name: string, status: string, source: string, task: string}[], logicalRows: number, appIconLoaded: boolean}} */
  static #report(iconLoaded) {
    const canvas = document.querySelector('.resource-chart');
    return {capturedAt: new Date().toISOString(), native: document.body.dataset.platform,
      section: this.#currentSection(),
      commandResources: Array.from(document.querySelectorAll(this.#COMMAND_RESOURCE_SELECTOR), /** @param {Element} element @returns {{binding: string | undefined, text: string}} */ element => ({binding: /** @type {HTMLElement} */ (element).dataset.group, text: this.#text(element)})),
      qaStage: document.body.dataset.nativeQaStage, qaBefore: document.body.dataset.nativeQaBefore,
      samples: this.#text(document.querySelector(this.#SAMPLE_SELECTOR)), sampleCount: this.#samples(),
      width: innerWidth, height: innerHeight, horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
      documentScroll: document.documentElement.scrollHeight > innerHeight, live: this.#live(),
      groups: document.querySelectorAll('.session-process-pick').length,
      graphGroups: document.querySelectorAll(this.#GROUP_SELECTOR).length, graphNodes: document.querySelectorAll(this.#GRAPH_NODE_SELECTOR).length, graphTabBounds: this.#tabBounds(),
      tableLayout: this.#grid(this.#TABLE_GRID, this.#TABLE_CARD), contextTableLayout: this.#grid('.context-resource-grid', this.#TABLE_CARD), historyLayout: this.#grid(this.#HISTORY_GRID, this.#HISTORY_CARD), contextHistoryLayout: this.#grid('.context-history-grid', this.#HISTORY_CARD),
      interaction: this.#interaction(), chartPoints: this.#points(canvas), chartUnit: this.#chartUnit(canvas),
      sessionContainers: document.querySelectorAll('.session-card').length, flatAgentCards: document.querySelectorAll('.session-grid > .agent-row').length,
      historyCards: document.querySelectorAll(this.#HISTORY_CARD).length, runtimeCards: document.querySelectorAll('.runtime-card').length,
      activityCards: document.querySelectorAll(this.#ACTION_ROW_SELECTOR).length, sidebarOverflow: this.#sidebarOverflow(),
      summaryBounds: this.#summaryBounds(), logicalRecords: this.#logicalRecords(), logicalRows: document.querySelectorAll(this.#AGENT_ROW_SELECTOR).length, appIconLoaded: iconLoaded, diagnostics: this.#diagnostics()};
  }

  /** @returns {{path: string, elements: number, document: Record<string, string | undefined>, failures: {tag: string, native: string | undefined, chart: string | undefined, mount: string | null}[], timing: {paint: [string, number][], marks: [string, number][], slow: [string, number, number][]}, panel: {view: string | undefined, session: string | undefined, html: string | undefined}}} */
  static #diagnostics() {
    const failures = Array.from(document.querySelectorAll('[data-native-failure],[data-chart-failure],[data-tachyon-mount-error]'),
      /** @param {Element} element @returns {{tag: string, native: string | undefined, chart: string | undefined, mount: string | null}} */ element => ({tag: element.tagName.toLowerCase(),
        native: /** @type {HTMLElement} */ (element).dataset.nativeFailure, chart: /** @type {HTMLElement} */ (element).dataset.chartFailure, mount: element.getAttribute('data-tachyon-mount-error')}));
    const entries = /** @param {string} type @returns {PerformanceEntry[]} */ type => globalThis.performance?.getEntriesByType?.(type) ?? [];
    const timing = {paint: entries('paint').map(/** @param {PerformanceEntry} entry @returns {[string, number]} */ entry => [entry.name, Math.round(entry.startTime)]),
      marks: entries('mark').map(/** @param {PerformanceEntry} entry @returns {[string, number]} */ entry => [entry.name, Math.round(entry.startTime)]),
      slow: entries('resource').filter(/** @param {PerformanceEntry} entry @returns {boolean} */ entry => entry.duration > 200).map(
        /** @param {PerformanceEntry} entry @returns {[string, number, number]} */ entry => [String(entry.name).split('/').pop() ?? '', Math.round(entry.startTime), Math.round(entry.duration)])};
    const panel = /** @type {HTMLElement | null} */ (document.querySelector?.(this.#PANEL_SELECTOR) ?? null);
    const panelState = {view: panel?.dataset.sessionView, session: panel?.dataset.session, html: panel?.innerHTML.slice(0, 2500)};
    return {path: globalThis.location?.pathname ?? '', elements: document.getElementsByTagName?.('*').length ?? 0, document: {...document.documentElement?.dataset}, failures, timing, panel: panelState};
  }

  /** @returns {{id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}[]} */
  static #tabBounds() {
    return Array.from(document.querySelectorAll(this.#GRAPH_TAB_SELECTOR)).filter(/** @param {Element} tab @returns {boolean} */ tab => this.#visible(tab)).map(/** @param {Element} tab @returns {{id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}} */ tab => this.#tabBound(tab));
  }

  /** @param {Element} tab @returns {{id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}} */
  static #tabBound(tab) {
    return {...this.#elementTextBound(tab), id: String(/** @type {HTMLElement} */ (tab).dataset.sessionTab),
      appearance: getComputedStyle(tab).appearance};
  }

  /** @param {string} check @returns {void} */
  static #assertNodeLabels(check) {
    if (check !== this.#TREE) return;
    this.#requireNodeLabels(this.#nodeLabelBounds());
  }

  /** @returns {{top: number, bottom: number, textTop: number, textBottom: number}[]} */
  static #nodeLabelBounds() {
    return Array.from(document.querySelectorAll(this.#GRAPH_NODE_SELECTOR)).map(
      /** @param {Element} node @returns {Element | null} */ node => node.querySelector('.graph-name')
    ).filter(/** @param {Element | null} label @returns {label is Element} */ label => label !== null).map(this.#elementTextBound.bind(this));
  }

  /** @param {{top: number, bottom: number, textTop: number, textBottom: number}[]} labels @returns {void} */
  static #requireNodeLabels(labels) {
    const ready = [labels.length > this.#EMPTY_COUNT, labels.every(/** @param {{top: number, bottom: number, textTop: number, textBottom: number}} label @returns {boolean} */ label => [label.textTop >= label.top, label.textBottom <= label.bottom].every(Boolean))].every(Boolean);
    if (!ready) throw new Error('Native QA process node titles are missing or clipped.');
  }

  /** @returns {{text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}[]} */
  static #summaryBounds() {
    return Array.from(document.querySelectorAll('.summary-grid .metric-label')).filter(/** @param {Element} label @returns {boolean} */ label => this.#visible(label)).map(/** @param {Element} label @returns {{text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}} */ label => this.#summaryBound(label));
  }

  /** @param {Element} label @returns {{text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}} */
  static #summaryBound(label) {
    return {...this.#elementTextBound(label), text: this.#text(label), overflow: getComputedStyle(label).overflow};
  }

  /** @param {Element} label @returns {{top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string}} */
  static #elementTextBound(label) {
    const rectangle = label.getBoundingClientRect();
    const range = document.createRange();
    range.selectNodeContents(label);
    const text = range.getBoundingClientRect();
    const style = getComputedStyle(label);
    return {top: rectangle.top, bottom: rectangle.bottom,
      textTop: text.top, textBottom: text.bottom, height: rectangle.height, lineHeight: style.lineHeight};
  }

  /** @returns {{id: string, name: string, status: string, source: string, task: string, sessionId: string | undefined, taskId: string | undefined, currentAction: string, commandResourceCount: number}[]} */
  static #logicalRecords() {
    return Array.from(document.querySelectorAll(this.#AGENT_ROW_SELECTOR)).map(this.#logicalRecord.bind(this));
  }

  /** @param {Element} row @returns {{id: string, name: string, status: string, source: string, task: string, sessionId: string | undefined, taskId: string | undefined, currentAction: string, commandResourceCount: number}} */
  static #logicalRecord(row) {
    const data = /** @type {HTMLElement} */ (row).dataset;
    return {id: data.agent ?? this.#EMPTY_TEXT, name: this.#text(row.closest('.session-card')?.querySelector('h3') ?? null),
      status: data.status ?? this.#EMPTY_TEXT, source: data.source ?? this.#EMPTY_TEXT,
      task: this.#text(row.querySelector('.agent-task')), sessionId: data.session, taskId: data.task,
      currentAction: this.#text(row.querySelector('.agent-tool-name')),
      commandResourceCount: row.querySelectorAll(this.#COMMAND_RESOURCE_SELECTOR).length};
  }

  /** @param {string} gridSelector @param {string} cardSelector @returns {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} */
  static #grid(gridSelector, cardSelector) {
    const grid = document.querySelector(gridSelector);
    const columns = this.#columns(grid);
    const cards = Array.from(document.querySelectorAll(gridSelector + ' ' + cardSelector)).filter(
      /** @param {Element} card @returns {boolean} */
      card => this.#visible(card)
    ).slice(0, this.#MAXIMUM_COLUMNS).map(
      /** @param {Element} card @returns {{x: number, y: number, width: number, height: number}} */
      card => this.#rectangle(card)
    );
    return {columns, cards};
  }

  /** @param {Element | null} grid @returns {number[]} */
  static #columns(grid) {
    if (grid === null) return [];
    return getComputedStyle(grid).gridTemplateColumns.split(/\s+/).map(Number.parseFloat).filter(Number.isFinite);
  }

  /** @param {Element} element @returns {boolean} */
  static #visible(element) {
    const rectangle = element.getBoundingClientRect();
    return [rectangle.width > this.#EMPTY_COUNT, rectangle.height > this.#EMPTY_COUNT].every(Boolean);
  }

  /** @param {Element} element @returns {{x: number, y: number, width: number, height: number}} */
  static #rectangle(element) {
    const value = element.getBoundingClientRect();
    return {x: value.x, y: value.y, width: value.width, height: value.height};
  }

  /** @returns {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} */
  static #interaction() {
    const text = document.body.dataset.qaBaseline;
    if (!text) return null;
    const baseline = JSON.parse(text);
    const scroll = Array.from(document.querySelectorAll(this.#SCROLL_SELECTOR)).find(
      /** @param {Element} element @returns {boolean} */
      element => /** @type {HTMLElement} */ (element).dataset.group === baseline.scroll?.id
    );
    return {baseline, current: JSON.parse(document.body.dataset.qaCurrent ?? '[]'), scroll: this.#scrollRecord(this.#element(scroll))};
  }

  /** @param {{horizontalOverflow: boolean, documentScroll: boolean, sidebarOverflow: boolean, tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextTableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextHistoryLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, sampleCount: number, live: string}} report @param {{check: string, width: number}} input @returns {void} */
  static #assertReport(report, input) {
    const value = report;
    this.#requirePage([!value.horizontalOverflow, !value.documentScroll, !value.sidebarOverflow].every(Boolean));
    this.#assertCheck(value, input);
  }

  /** @param {boolean} success @returns {void} */
  static #requirePage(success) {
    if (!success) throw new Error('Native QA found page or sidebar overflow.');
  }

  /** @param {{horizontalOverflow: boolean, documentScroll: boolean, sidebarOverflow: boolean, tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextTableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, contextHistoryLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, sampleCount: number, live: string}} report @param {{check: string, width: number}} input @returns {void} */
  static #assertCheck(report, input) {
    switch (input.check) {
      case 'current-work-completed': this.#requireHistoryLabels(); return;
      case this.#HISTORY: this.#requireDrawerGrid(report.historyLayout); return;
      case this.#ZOOM: this.#requireZoom(report.interaction); return;
      case this.#POLLS: this.#requireZoom(report.interaction); this.#requireScroll(report.interaction); return;
      case this.#HOLD: this.#requirePaused(report.sampleCount, report.live); return;
      default: return;
    }
  }

  /** @returns {void} */
  static #requireHistoryLabels() {
    if (!this.#historyComplete()) throw new Error('Native QA completion no longer matches its exact reported action.');
  }

  /** @param {{columns: number[], cards: {width: number}[]}} grid @returns {void} */
  static #requireDrawerGrid(grid) {
    this.#requireSessionCapacity(grid, this.#SINGLE_COLUMN);
    this.#requireMeasuredCards(grid.cards.length);
  }

  /** @param {{columns: number[], cards: {width: number}[]}} grid @param {number} expected @returns {void} */
  static #requireSessionCapacity(grid, expected) {
    const ready = [grid.columns.length === expected, this.#cardsFitColumns(grid)].every(Boolean);
    this.#requireLayout(ready, expected, grid.columns.length);
  }

  /** @param {number} count @returns {void} */
  static #requireMeasuredCards(count) {
    if (count === this.#EMPTY_COUNT) throw new Error('Native QA needs actual session or context cards for this view.');
  }

  /** @param {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} layout @param {number} expected @returns {void} */
  static #requireGrid(layout, expected) {
    const ready = this.#gridReady(layout, expected);
    this.#requireLayout(ready, expected, layout.columns.length);
  }

  /** @param {{columns: number[], cards: {width: number}[]}} layout @returns {boolean} */
  static #cardsFitColumns(layout) {
    return layout.cards.every(/** @param {{width: number}} card @returns {boolean} */ card => Math.abs(card.width - layout.columns[0]) <= this.#SIZE_TOLERANCE);
  }

  /** @param {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} layout @param {number} expected @returns {boolean} */
  static #gridReady(layout, expected) {
    const sameRow = this.#sameRow(layout.cards);
    return [layout.columns.length === expected, layout.cards.length >= expected,
      this.#equalCardWidths(layout.cards), this.#expectedRow(expected, sameRow)].every(Boolean);
  }

  /** @param {{y: number}[]} cards @returns {boolean} */
  static #sameRow(cards) {
    if (cards.length < this.#REQUIRED_GROUPS) return false;
    return Math.abs(cards[0].y - cards[1].y) <= this.#SIZE_TOLERANCE;
  }

  /** @param {number} expected @param {boolean} sameRow @returns {boolean} */
  static #expectedRow(expected, sameRow) {
    if (expected === this.#SINGLE_COLUMN) return true;
    return sameRow;
  }

  /** @param {{width: number}[]} cards @returns {boolean} */
  static #equalCardWidths(cards) {
    return cards.every(
      /** @param {{width: number}} card @returns {boolean} */
      card => Math.abs(card.width - cards[0].width) <= this.#SIZE_TOLERANCE
    );
  }

  /** @param {boolean} ready @param {number} expected @param {number} actual @returns {void} */
  static #requireLayout(ready, expected, actual) {
    if (!ready) throw new Error('Native QA grid expected ' + expected + ' equal columns; measured ' + actual + '.');
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {void} */
  static #requireZoom(interaction) {
    const ready = this.#zoomReady(interaction);
    this.#requireZoomReady(ready, interaction);
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {boolean} */
  static #zoomReady(interaction) {
    if (interaction === null) return false;
    const baseline = interaction.baseline;
    return [baseline.before.length === this.#REQUIRED_GROUPS, baseline.after.length === this.#REQUIRED_GROUPS,
      baseline.before[0]?.id === baseline.after[0]?.id, baseline.before[1]?.id === baseline.after[1]?.id,
      baseline.before[0]?.scale !== baseline.after[0]?.scale, baseline.before[1]?.scale === baseline.after[1]?.scale,
      JSON.stringify(interaction.current) === JSON.stringify(baseline.after)].every(Boolean);
  }

  /** @param {boolean} ready @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {void} */
  static #requireZoomReady(ready, interaction) {
    if (!ready) {
      throw new Error('Native QA independent zoom or zoom preservation failed. ' + JSON.stringify(interaction));
    }
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {void} */
  static #requireScroll(interaction) {
    const ready = this.#scrollReady(interaction);
    this.#requireScrollReady(ready);
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {boolean} */
  static #scrollReady(interaction) {
    if (interaction === null) return false;
    return JSON.stringify(interaction.scroll) === JSON.stringify(interaction.baseline.scroll);
  }

  /** @param {boolean} ready @returns {void} */
  static #requireScrollReady(ready) {
    if (!ready) throw new Error('Native QA group pan changed across three measured updates.');
  }

  /** @param {number} samples @param {string} live @returns {void} */
  static #requirePaused(samples, live) {
    const ready = [samples === Number(document.body.dataset.qaPausedSamples), live.toLowerCase().includes(this.#PAUSED)].every(Boolean);
    this.#requirePauseReady(ready);
  }

  /** @param {boolean} ready @returns {void} */
  static #requirePauseReady(ready) {
    if (!ready) throw new Error('Native QA paused sample count changed during the hold.');
  }

  /** @returns {number} */
  static #samples() {
    return Number.parseFloat(this.#text(document.querySelector(this.#SAMPLE_SELECTOR)));
  }

  /** @returns {string} */
  static #live() {
    return this.#text(document.querySelector('.live-state'));
  }

  /** @param {Element | null} element @returns {string} */
  static #text(element) {
    if (element === null) return this.#EMPTY_TEXT;
    return String(element.textContent).trim();
  }

  /** @param {Element | null} canvas @returns {number | null} */
  static #points(canvas) {
    const text = this.#attribute(canvas, 'data-points');
    return this.#number(text);
  }

  /** @param {Element | null} element @param {string} name @returns {string | null} */
  static #attribute(element, name) {
    if (element === null) return null;
    return element.getAttribute(name);
  }

  /** @param {string | null} text @returns {number | null} */
  static #number(text) {
    const number = Number.parseFloat(String(text));
    if (!Number.isFinite(number)) return null;
    return number;
  }

  /** @param {Element | null} canvas @returns {string | null} */
  static #chartUnit(canvas) {
    const constructor = /** @type {Window & {Chart?: {getChart: (canvas: Element | null) => {options: {scales: {y: {title: {text: string}}}}} | undefined}}} */ (window).Chart;
    const chart = constructor?.getChart(canvas);
    return this.#unit(chart);
  }

  /** @param {{options: {scales: {y: {title: {text: string}}}}} | undefined} chart @returns {string | null} */
  static #unit(chart) {
    if (chart === undefined) return null;
    return chart.options.scales.y.title.text;
  }

  /** @returns {boolean} */
  static #sidebarOverflow() {
    const sidebar = document.querySelector('.sidebar');
    if (sidebar === null) return false;
    return sidebar.scrollHeight > sidebar.clientHeight;
  }
}
