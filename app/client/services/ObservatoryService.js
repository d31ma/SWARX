import { ScriptUtils } from '../utils/ScriptUtils.js';

@Service
export default class ObservatoryService {
    /** @type {number} */
    static _POLL_MILLISECONDS = 2000;
    /** @type {number} */
    static _FAILURE_CHARACTER_LIMIT = 180;
    /** @type {string} */
    static _EMPTY_TEXT = '';
    /** @type {string} */
    static _LOADING_VIEW = 'loading';
    /** @type {string} */
    static _LIST_VIEW = 'list';
    /** @type {string} */
    static _PANEL_SELECTOR = '.session-panel';
    /** @type {string} */
    static _CARD_SELECTOR = '.session-card-open';
    /** @type {string} */
    static _COLLECTOR_UNAVAILABLE = 'Local measurements unavailable. Retrying automatically; last values may be older.';
    /** @type {import('/services/native/monitoring/SnapshotService.js').default | undefined} */
    @Needs
    snapshot;
    /** @type {TacTaskScheduler | undefined} */
    @Needs
    taskScheduler;
    /** @type {AbortSignal | undefined} */
    @Needs
    lifecycle;
    /** @type {HTMLElement | undefined} */
    @Needs
    view;
    /** @type {import('/services/browser/PresentationService.js').default | undefined} */
    @Needs
    presentation;
    /** @type {TacWindowControls | undefined} */
    @Needs
    windowControls;
    /** @type {import('/models/ObservatoryModel.ts').ObservatoryModel | undefined} */
    model;
    /** @type {string} */
    #selected = ObservatoryService._EMPTY_TEXT;
    /** @type {string} */
    status = 'Connecting';
    /** @type {string} */
    series = 'cpu';
    /** @type {string} */
    selectedSession = ObservatoryService._EMPTY_TEXT;
    /** @type {string} */
    sessionTab = 'resources';
    /** @type {string} */
    sessionView = ObservatoryService._LIST_VIEW;
    /** @type {string[]} */
    #expandedSessions = [];
    /** @type {number} */
    #selectedProcessNumber = 0;
    /** @type {number | null} */
    #sessionProcessNumber = null;
    /** @type {string | null} */
    #sessionProcessStarted = null;
    /** @type {number} */
    #revision = 0;
    /** @type {boolean} */
    #queued = false;
    /** @type {boolean} */
    #busy = false;
    /** @type {string} */
    problem = ObservatoryService._EMPTY_TEXT;
    /** @type {ReturnType<typeof setInterval> | null} */
    #timer = null;
    /** @type {HTMLElement | undefined} */
    #host = undefined;

    /** @returns {void} */
    @OnStart
    initialize() {
        ScriptUtils.load();
        // The title bar keeps its close, minimize and full-screen controls but shows no title text.
        this.#windows().setTitle(ObservatoryService._EMPTY_TEXT);
        // Section components share this page singleton, and Tac rebinds `view` to each newly
        // activated component host. Keep the observatory panel host that started the service.
        this.#host = this.#provider(this.view);
        this.#presentation().watch(this.#element(), this.#signal(),
            /** @returns {import('/models/ViewSelection.ts').ViewSelection} */ () => this.#selection());
        this.#schedule();
        this.#timer = setInterval(/** @returns {void} */ () => this.#poll(), ObservatoryService._POLL_MILLISECONDS);
    }

    /** @returns {void} */
    @OnStop
    dispose() {
        this.#revision += 1;
        this.#queued = false;
        this.#clearTimer();
        this.#presentation().dispose();
    }

    /** @param {Event} _event @param {string} identifier @returns {void} */
    #select(_event, identifier) {
        this.#selected = identifier;
        this.#search();
    }

    /** @param {Event} _event @param {number} identifier @param {string} started @returns {void} */
    selectSessionProcess(_event, identifier, started) {
        this.#sessionProcessNumber = identifier;
        this.#sessionProcessStarted = started;
        this.#search();
    }

    /** @param {Event} _event @param {string} mode @returns {void} */
    setSeries(_event, mode) {
        this.series = mode;
        void this.#renderIfReady();
    }

    // Opens the session panel for the exact ActiveSessionUtils key; a process pick belongs to one session.
    /** @param {Event} _event @param {string} key @returns {void} */
    openSession(_event, key) {
        this.selectedSession = key;
        this.sessionView = ObservatoryService._LOADING_VIEW;
        this.#clearSessionProcess();
        this.#search();
        this.#focusAfterRender(key, ObservatoryService._PANEL_SELECTOR);
    }

    /** @param {Event} _event @returns {void} */
    closeSession(_event) {
        const key = this.selectedSession;
        this.selectedSession = ObservatoryService._EMPTY_TEXT;
        this.sessionView = ObservatoryService._LIST_VIEW;
        this.#clearSessionProcess();
        this.#search();
        this.#focusAfterRender(key, ObservatoryService._CARD_SELECTOR);
    }

    /** @returns {void} */
    #clearSessionProcess() {
        this.#sessionProcessNumber = null;
        this.#sessionProcessStarted = null;
    }

    /** @param {Event} _event @param {string} key @returns {void} */
    toggleSession(_event, key) {
        this.#expandedSessions = this.#toggled(key, this.#without(key));
        this.#search();
    }

    /** @param {Event} _event @param {string} tab @returns {void} */
    #setSessionTab(_event, tab) {
        this.sessionTab = tab;
        void this.#renderIfReady();
    }

    /** @returns {void} */
    #search() {
        this.#revision += 1;
        this.#schedule();
    }

    /** @param {string} key @returns {string[]} */
    #without(key) {
        return this.#expandedSessions.filter(/** @param {string} item @returns {boolean} */ item => item !== key);
    }

    /** @param {string} key @param {string[]} without @returns {string[]} */
    #toggled(key, without) {
        if (this.#expandedSessions.includes(key)) {
            return without;
        }
        return [...this.#expandedSessions, key];
    }

    /** @param {Event} event @returns {void} */
    #closeOpenSession(event) {
        if (this.selectedSession === ObservatoryService._EMPTY_TEXT) {
            return;
        }
        this.closeSession(event);
    }

    /** @param {string} identifier @param {string} selector @returns {void} */
    #focusAfterRender(identifier, selector) {
        void this.#focusRendered(identifier, selector)
            .catch(/** @param {unknown} error @returns {void} */ error => this.#chartFailure(error));
    }

    /** @param {string} identifier @param {string} selector @returns {Promise<void>} */
    async #focusRendered(identifier, selector) {
        await this.#scheduler().render();
        this.#presentation().focusSession(identifier, selector);
    }

    /** @returns {import('/models/ViewSelection.ts').ViewSelection} */
    #selection() {
        return {
            group: this.#detail(),
            mode: this.series,
            model: this.model,
            pid: this.#selectedProcessNumber,
            session: this.selectedSession,
            sessionPid: this.#sessionProcessNumber,
            sessionStarted: this.#sessionProcessStarted,
            showTab: /** @param {Event} event @param {string} tab @returns {void} */
                (event, tab) => this.#setSessionTab(event, tab),
            close: /** @param {Event} event @returns {void} */ event => this.#closeOpenSession(event),
            pick: /** @param {Event} event @param {string} group @param {number} identifier @returns {void} */
                (event, group, identifier) => this.#selectProcess(event, group, identifier)
        };
    }

    /** @returns {import('/models/RuntimeView.ts').RuntimeView | null} */
    #detail() {
        if (this.model === undefined) {
            return null;
        }
        return this.model.detail;
    }

    /** @returns {Promise<void>} */
    async #renderIfReady() {
        if (this.model === undefined) {
            return;
        }
        await this.#renderCharts();
    }

    /** @returns {Promise<void>} */
    async #renderCharts() {
        await this.#scheduler().render();
        this.#presentation().render(this.#element(), this.#selection());
    }

    /** @param {Event} event @param {string} group @param {number} identifier @returns {void} */
    #selectProcess(event, group, identifier) {
        this.#selectedProcessNumber = identifier;
        this.#select(event, group);
    }

    /** @returns {void} */
    #clearTimer() {
        if (this.#timer === null) {
            return;
        }
        clearInterval(this.#timer);
        this.#timer = null;
    }

    /** @returns {void} */
    #poll() {
        if (this.#signal().aborted) {
            return;
        }
        this.#schedule();
    }

    /** @returns {void} */
    #schedule() {
        this.#queued = true;
        this.#launch();
    }

    /** @returns {void} */
    #launch() {
        if (this.#busy) {
            return;
        }
        this.#busy = true;
        void this.#drain();
    }

    /** @returns {Promise<void>} */
    async #drain() {
        while (this.#queued) {
            this.#queued = false;
            const revision = this.#revision;
            const selection = this.#nativeSelection();
            await this.#collect(revision, selection);
        }
        this.#busy = false;
    }

    // Tac reads native arguments by Rust field name, so these keys are the exact NativeViewSelection fields.
    /** @returns {import('/models/NativeViewSelection.ts').NativeViewSelection} */
    #nativeSelection() {
        return { selected: this.#selected, query: ObservatoryService._EMPTY_TEXT, pid: this.#selectedProcessNumber,
            selected_session: this.selectedSession, session_pid: this.#sessionProcessNumber,
            session_started: this.#sessionProcessStarted, expanded_sessions: [...this.#expandedSessions] };
    }

    /** @param {number} revision @param {import('/models/NativeViewSelection.ts').NativeViewSelection} selection @returns {Promise<void>} */
    #collect(revision, selection) {
        return Promise.resolve()
            .then(/** @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
                () => this.#snapshotProvider().read(selection))
            .then(/** @param {import('/models/ObservatoryModel.ts').ObservatoryModel} model @returns {void} */
                model => this.#accept(model, revision),
                /** @param {unknown} error @returns {void} */ error => this.#failure(error, revision));
    }

    /** @param {import('/models/ObservatoryModel.ts').ObservatoryModel} model @param {number} revision @returns {void} */
    #accept(model, revision) {
        if (revision !== this.#revision) {
            return;
        }
        this.#acceptCurrent(model);
    }

    /** @param {import('/models/ObservatoryModel.ts').ObservatoryModel} model @returns {void} */
    #acceptCurrent(model) {
        this.model = model;
        this.sessionView = model.sessionView;
        this.#selected = model.selected;
        this.#sessionProcessNumber = this.#processPid(model.selectedSessionProcess);
        this.#sessionProcessStarted = this.#processStart(model.selectedSessionProcess);
        this.status = model.liveStatus;
        this.problem = ObservatoryService._EMPTY_TEXT;
        delete this.#element().dataset.nativeFailure;
        void this.#renderCharts().catch(/** @param {unknown} error @returns {void} */ error => this.#chartFailure(error));
    }

    /** @param {import('/models/ProcessRow.ts').ProcessRow | null | undefined} process @returns {number | null} */
    #processPid(process) {
        return process?.pid ?? null;
    }

    /** @param {import('/models/ProcessRow.ts').ProcessRow | null | undefined} process @returns {string | null} */
    #processStart(process) {
        return process?.started ?? null;
    }

    /** @param {unknown} error @param {number} revision @returns {void} */
    #failure(error, revision) {
        if (revision !== this.#revision) {
            return;
        }
        const reason = this.#failureMessage(error);
        this.#element().dataset.nativeFailure = reason;
        this.problem = ObservatoryService._COLLECTOR_UNAVAILABLE + ' ' + reason;
        this.status = 'Disconnected';
        // Without a render the last snapshot stays on screen and the failure is invisible.
        void this.#scheduler().render().catch(/** @param {unknown} renderError @returns {void} */ renderError => this.#chartFailure(renderError));
    }

    /** @param {unknown} error @returns {void} */
    #chartFailure(error) {
        this.#element().dataset.chartFailure = this.#failureMessage(error);
    }

    /** @param {unknown} error @returns {string} */
    #failureMessage(error) {
        return String(error).slice(0, ObservatoryService._FAILURE_CHARACTER_LIMIT);
    }

    /** @returns {import('/services/native/monitoring/SnapshotService.js').default} */
    #snapshotProvider() {
        return this.#provider(this.snapshot);
    }

    /** @returns {import('/services/browser/PresentationService.js').default} */
    #presentation() {
        return this.#provider(this.presentation);
    }

    /** @returns {TacWindowControls} */
    #windows() {
        return this.#provider(this.windowControls);
    }

    /** @returns {TacTaskScheduler} */
    #scheduler() {
        return this.#provider(this.taskScheduler);
    }

    /** @returns {AbortSignal} */
    #signal() {
        return this.#provider(this.lifecycle);
    }

    /** @returns {HTMLElement} */
    #element() {
        return this.#provider(this.#host);
    }

    /** @template T @param {T | undefined} value @returns {T} */
    #provider(value) {
        if (value === undefined) {
            throw new Error('Required browser provider is unavailable.');
        }
        return value;
    }
}
