import {QaPathUtils} from './QaPathUtils.mjs';
import { OverviewFixtureClient } from './OverviewFixtureClient.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import {BrowserBundleProvenanceUtils} from './utils/BrowserBundleProvenanceUtils.mjs';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

/** @typedef {{x: number, y: number, width: number, height: number}} OfflineRectangle */
/** @typedef {{width: number, height: number, documentX: boolean, documentY: boolean, footerBottom: number, summaryCards: number}} OfflineShell */
/** @typedef {{text: string | null, card: string}} OfflineClipping */

/** @typedef {import('../../app/client/models/ResourceTotals.ts').ResourceTotals} OfflineTotals */
/** @typedef {import('../../app/client/models/NativeViewSelection.ts').NativeViewSelection} NativeSelection */
/** @typedef {import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel} Overview */
/** @typedef {import('../../app/client/models/SnapshotTransfer.ts').SnapshotTransfer} SnapshotTransfer */
/** @typedef {import('../../app/client/models/SnapshotPage.ts').SnapshotPage} SnapshotPage */
/** @typedef {Omit<import('../../app/client/models/LogicalAgent.ts').LogicalAgent, 'session' | 'task'> & {session: import('../../app/client/models/SessionLink.ts').SessionLink, task: import('../../app/client/models/TaskRecord.ts').TaskRecord}} OfflineLogical */
/** @typedef {import('../../app/client/models/ActivityRecord.ts').ActivityRecord} OfflineAction */
/** @typedef {{pid:number,parent:number,started:string,name:string,cpu_percent:number,resident_bytes:number,received_per_second:null,sent_per_second:null,disk_read_per_second:number,disk_write_per_second:number}} OfflineProcess */
/** @typedef {{cpu_percent:number,resident_bytes:number,cpu_covered_processes:number,memory_covered_processes:number,disk_covered_processes:number,received_per_second:number|null,sent_per_second:number|null,disk_read_per_second:number,disk_write_per_second:number,network_status:string,network_covered_processes:number,totals:OfflineTotals}} OfflineMetrics */
/** @typedef {{id:string,pid:number,started:string,family:string,name:string,status:string,scope:string,shared:boolean,instrumented:boolean,identity_evidence:string,members:number[],processes:OfflineProcess[],task_status:string,metrics:OfflineMetrics,history:(number|null)[][]}} OfflineGroup */
/** @typedef {{id:string,name:string,family:string,captured_at:number,session_id:string,session_name:string,records:OfflineAction[]}} OfflineActivity */
/** @typedef {{agent_id:string,session_id:string,task_id:string,execution_id:string}} OfflineCommandOwner */
/** @typedef {{pid:number,parent:number,started:string,name:string,cpu_percent:number|null,resident_bytes:number|null,received_per_second:number|null,sent_per_second:number|null,disk_read_per_second:number|null,disk_write_per_second:number|null}} OfflineCommandProcess */
/** @typedef {{time:number,processes:OfflineCommandProcess[]}} OfflineTreeSample */
/** @typedef {{binding_id:string,generation:number,owner:OfflineCommandOwner,pid:number,started:string,sampled_at:number,scope:string,provenance:string,validation:string,coverage:string,metrics:OfflineCommandProcess,descendants:OfflineCommandProcess[],totals:OfflineTotals,history:(number|null)[][],tree_history:OfflineTreeSample[]}} OfflineCommandObservation */
/** @typedef {{id:string,run_id:string,generation:number,owner:OfflineCommandOwner,identity:{pid:number,started:string,user:number,executable:string},received_at:number,verified_at:number,lease_until:number,scope:string,provenance:string,validation:string}} OfflineCommandBinding */
/** @typedef {[OfflineCommandObservation, OfflineCommandBinding]} OfflineCommandLink */
/** @typedef {{sampled_at:number,started_at:number,groups:OfflineGroup[],logical_agents:OfflineLogical[],activity:OfflineActivity[],command_observations:OfflineCommandObservation[],command_bindings:OfflineCommandBinding[],totals:OfflineTotals,errors:string[],limitations:string[]}} OfflineFixture */
/** @typedef {OfflineFixture & {now:number,selection:NativeSelection}} OfflineRequest */
/** @typedef {{open:(selection:NativeSelection)=>Promise<SnapshotTransfer>,page:(id:number,index:number)=>Promise<SnapshotPage>,release:(id:number)=>Promise<boolean>}} OfflineNative */
/** @typedef {{op:string,service:string,name:string,args:unknown[]}} OfflineHostRequest */
/** @typedef {{options:{scales:{x:{max:number,min:number,title:{text:string}},y:{title:{text:string}}}},data:{datasets:import('../../app/client/models/ChartDataset.ts').ChartDataset[]},tooltip:{opacity:number}}} OfflineChart */
/** @typedef {{getChart:(canvas:Element|null)=>OfflineChart|undefined}} OfflineChartLibrary */
/** @typedef {typeof globalThis & {offlineFixture:OfflineFixture,offlineNative:OfflineNative,offlineRustOverview:(request:OfflineRequest)=>Promise<Overview>,Chart?:OfflineChartLibrary}} OfflineBrowser */
/** @typedef {{pid:number,identity:string,x:number,y:number,text:string|null}} OfflineNode */
/** @typedef {{nodes:OfflineNode[],edges:number[][],trunks:number}} OfflineTree */
/** @typedef {{zoom:string|null,canvasWidth:number,viewWidth:number,nodes:Omit<OfflineNode,'text'>[],edges:(string|undefined)[][]}} OfflineGraphGeometry */
/** @typedef {{unit:string,window:number,points:number,height:number,width:number,title:string,gaps:boolean,spanGaps:boolean}} OfflineChartGeometry */
/** @typedef {{top:number,bottom:number,left:number,right:number}} OfflineTextBounds */
/** @typedef {OfflineTextBounds & {label:string|null,height:number,appearance:string,text:OfflineTextBounds[]}} OfflineTab */
/** @typedef {{before:number,after:number,viewport:{width:number,height:number},selection:string|null,focused:boolean}} ListContinuity */
/** @typedef {{list:OfflineRectangle,panel:OfflineRectangle,listVisible:boolean,backVisible:boolean}} PanelLayout */
/** @typedef {{width:number,height:number,availableWidth:number,availableHeight:number}} OfflineFit */
/** @typedef {{width:number,bottom:number,groupWidth:number,scrollWidth:number,scrollHeight:number,scrollBottom:number}} OfflineTreeArea */
/** @typedef {{alerts:string[],values:string[],footer:string}} OfflineDisconnected */

class OfflineUiTest {
  /** @type {URL} */
  static ROOT = new URL('../../', import.meta.url);
  /** @type {URL} */
  static EMITTED = QaPathUtils.directory('OBSERVATORY_WEB_BUNDLE');
  /** @type {URL} */
  static BUILD_SOURCE = QaPathUtils.file('OBSERVATORY_BUILD_SOURCE');
  /** @type {string} */
  static ORIGIN = 'https://offline-ui.invalid';
  /** @type {string} */
  static CHROME = QaPathUtils.external('OBSERVATORY_BROWSER_EXECUTABLE');
  /** @type {string} */
  static LINKED_SESSION = 'Codex:offline-test-session-0';
  /** @type {string} */
  static SECOND_LINKED_SESSION = 'Claude:offline-test-session-1';
  /** @type {string} */
  static UNLINKED_SESSION = 'Codex:offline-test-session-2';
  /** @type {number} */
  static LINKED_ROOT = 91000;
  /** @type {number} */
  static SECOND_LINKED_ROOT = 91100;
  /** @type {number} */
  static SELECTED_CHILD = 91001;
  /** @type {number} */
  static LINKED_PROCESSES = 7;
  /** @type {number} */
  static SECOND_LINKED_PROCESSES = 3;
  /** @type {number} */
  static HISTORY_POINTS = 61;
  /** @type {number} */
  static HISTORY_SECONDS = 120;
  /** @type {number} */
  static SAMPLE_SECONDS = 2;
  /** @type {number} */
  static GAP_EVERY = 13;
  /** @type {number} */
  static MEBIBYTE = 1048576;
  /** @type {number} */
  static BIRTH_SECONDS = 1790000000;
  /** @type {number} */
  static POLL_MILLISECONDS = 100;
  /** @type {number} */
  static MINIMUM_TAB_HEIGHT = 40;
  /** @type {string} */
  static SESSION_CARD = '.session-card';
  /** @type {string} */
  static SESSION_TAB = '[role="tab"][data-session-tab]';
  /** @type {string} */
  static SELECTED_TAB = '[role="tab"][data-session-tab][aria-selected="true"]';
  /** @type {string} */
  static GRAPH_GROUP = '.graph-group';
  /** @type {string} */
  static GRAPH_NODE = '.graph-node';
  /** @type {string} */
  static ZOOM_VALUE = '.graph-zoom-value';
  /** @type {string} */
  static ZOOM_IN = '[data-zoom="in"]';
  /** @type {string} */
  static GRAPH_SCROLL = '.group-graph-scroll';
  /** @type {string} */
  static SELECTED_PROCESS = '.session-selected-process';
  /** @type {string} */
  static FOOTER_SAMPLES = '.main-footer > span:last-child';
  /** @type {string} */
  static SESSION_LIST = '.logical-scroll';
  /** @type {string} */
  static ACTION_ROW = '.action-history-row';
  /** @type {string} */
  static ELAPSED = '.action-elapsed';
  /** @type {string} */
  static RESOURCE_CHART = '.resource-chart';
  /** @type {string} */
  static RESOURCES_TAB = 'resources';
  /** @type {string} */
  static TREE_TAB = 'tree';
  /** @type {string} */
  static ACTIVITY_TAB = 'activity';
  /** @type {string} */
  static FIXTURE_SOURCE = 'synthetic fixture only';
  /** @type {string} */
  static FIXTURE_ACTION = 'fixture-action';
  /** @type {{width: number, height: number}} */
  static NORMAL = {width: 1240, height: 850};
  /** @type {{width: number, height: number}} */
  static SMALL = {width: 1000, height: 700};
  /** @type {{width: number, height: number}} */
  static WIDE = {width: 1680, height: 1000};
  /** @type {{kind: string, checks: unknown[], errors: string[], requests: string[], sources: Record<string, string>, success: boolean}} */
  static report = {kind: 'offline Chromium UI acceptance; synthetic native boundary; NOT native Mac QA', checks: [], errors: [], requests: [], sources: {}, success: false};

  /** @returns {Promise<void>} */
  static async run() {
    this.sources();
    const browser = await chromium.launch({executablePath: this.CHROME});
    try {
      await this.check(browser);
      this.requireStableSources();
      this.report.success = true;
    } catch (error) {
      this.report.errors.push(String(error));
      throw error;
    } finally {
      await browser.close();
      writeFileSync(QaPathUtils.evidence('offline-ui-report.json'), JSON.stringify(this.report, null, 2));
    }
    console.log('PASS: offline compiler-emitted UI, synthetic native I/O only; ' + this.report.checks.length + ' checks.');
  }

  /** @returns {void} */
  static sources() {
    mkdirSync(QaPathUtils.evidence(''), {recursive: true});
    this.report.sources = BrowserBundleProvenanceUtils.inspect(this.ROOT, this.EMITTED, this.BUILD_SOURCE);
  }

  /** @returns {void} */
  static requireStableSources() {
    BrowserBundleProvenanceUtils.requireStable(this.report.sources, this.ROOT, this.EMITTED, this.BUILD_SOURCE);
  }

  /** @param {import("playwright").Route} route @returns {Promise<void>} */
  static async route(route) {
    const url = new URL(route.request().url());
    this.report.requests.push(url.href);
    assert.equal(url.origin, this.ORIGIN, 'All browser traffic stays in intercepted offline origin.');
    const name = url.pathname;
    if (name === '/') {
      const html = readFileSync(new URL('index.html', this.EMITTED), 'utf8');
      return route.fulfill({contentType: 'text/html', body: html.replace('</head>', '<link rel="stylesheet" href="/shared/styles/app.css"></head>')});
    }
    const relative = name.startsWith('/.tachyon/') ? 'tachyon-runtime/' + name.slice('/.tachyon/'.length) : name.slice(1);
    const asset = new URL(relative, this.EMITTED);
    const source = name.startsWith('/shared/assets/') || name.startsWith('/shared/styles/') ? new URL('app/client' + name, this.ROOT) : asset;
    if (!existsSync(source)) {
      this.report.errors.push('Missing offline asset: ' + name);
      return route.fulfill({status: 404, body: 'Missing offline asset'});
    }
    /** @type {Record<string, string>} */
    const types = {'.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.json': 'application/json', '.webmanifest': 'application/manifest+json'};
    const extension = name.slice(name.lastIndexOf('.'));
    const body = readFileSync(source);
    return route.fulfill({contentType: types[extension] ?? 'application/octet-stream', body});
  }

  /** @returns {OfflineFixture} */
  static fixture() {
    const now = Date.now() / 1000;
    /** @type {OfflineTotals} */
    const totals = {cpu_seconds: 3.2, resident_bytes: 200 * this.MEBIBYTE, peak_resident_bytes: 250 * this.MEBIBYTE,
      received_bytes: 10240, sent_bytes: 2048, disk_read_bytes: 0, disk_written_bytes: 0, samples: 1};
    const groups = [0, 1, 2, 3].map(/** @param {number} index @returns {OfflineGroup} */ index => {
      const pid = 90000 + index * 100;
      const family = index % 2 ? 'Claude' : 'Codex';
      const processes = [0, 1, 2, 3, 4, 5, 6].map(/** @param {number} offset @returns {OfflineProcess} */ offset => ({pid: pid + offset, parent: offset === 0 ? 1 : offset < 4 ? pid : pid + 1,
        started: 'OFFLINE-TEST-START-' + (pid + offset), name: offset ? 'fixture-child-' + offset : 'fixture-' + family,
        cpu_percent: 2.5 + offset, resident_bytes: (20 + offset) * this.MEBIBYTE, received_per_second: null, sent_per_second: null,
        disk_read_per_second: 1024, disk_write_per_second: 2048}));
      return {id: 'offline-' + pid + '-start', pid, started: 'OFFLINE-TEST-START-' + pid, family,
        name: family + ' synthetic runtime ' + (index + 1), status: 'process-present', scope: 'shared-runtime', shared: true,
        instrumented: false, identity_evidence: 'synthetic test boundary', members: processes.map(/** @param {OfflineProcess} process @returns {number} */ process => process.pid), processes,
        task_status: 'unavailable', metrics: {cpu_percent: 35.5, resident_bytes: 200 * this.MEBIBYTE,
          cpu_covered_processes: 7, memory_covered_processes: 7, disk_covered_processes: 7,
          received_per_second: index === 1 ? null : 1024, sent_per_second: index === 1 ? null : 512,
          disk_read_per_second: 10240, disk_write_per_second: 2048,
          network_status: index === 1 ? 'unavailable' : 'partial', network_covered_processes: index === 1 ? 0 : 1, totals},
        history: Array.from({length: this.HISTORY_POINTS}, /** @param {unknown} _ @param {number} i @returns {(number|null)[]} */ (_, i) => [now - this.HISTORY_SECONDS + i * this.SAMPLE_SECONDS, i % this.GAP_EVERY === 0 ? null : 30 + Math.sin(i / 4) * 20,
          (200 + Math.sin(i / 8) * 12) * this.MEBIBYTE, index === 1 || i % this.GAP_EVERY === 0 ? null : 1024 * (1 + i % 5),
          index === 1 ? null : 512, i % this.GAP_EVERY === 0 ? null : 10240, 2048])};
    });
    const logical_agents = [0, 1, 2, 3, 4, 5].map(/** @param {number} index @returns {OfflineLogical} */ index => ({id: 'offline-agent-' + index,
      name: 'Synthetic QA agent ' + (index + 1), family: index % 2 ? 'Claude' : 'Codex',
      session: {id: 'offline-test-session-' + index, name: 'Offline test session ' + index, source: 'synthetic test boundary'},
      status: index === 4 ? 'completed' : 'running', source: this.FIXTURE_SOURCE, captured_at: now,
      feed_mode: index === 5 ? 'recovered' : 'events',
      task: {id: 'offline-task-' + index, name: 'Synthetic UI acceptance task', status: index === 4 ? 'completed' : 'running', depends_on: []},
      executions: [{id: 'offline-action-' + index, name: this.FIXTURE_ACTION, status: index === 4 ? 'completed' : 'running', started_at: 0,
        session_id: 'offline-test-session-' + index, task_id: 'offline-task-' + index, source: this.FIXTURE_SOURCE,
        exit_code: index === 4 ? 0 : null, duration_seconds: null}],
      age_seconds: index === 5 ? 240 : 0, stale: index === 5}));
    const activity = logical_agents.slice(0, 4).map(/** @param {OfflineLogical} agent @param {number} index @returns {OfflineActivity} */ (agent, index) => ({id: agent.id + ':' + agent.session.id, name: agent.name, family: agent.family, captured_at: now,
      session_id: agent.session.id, session_name: agent.session.name,
      records: [{id: 'offline-history-' + index, agent_id: agent.id, agent_name: agent.name, agent_family: agent.family, freshness: index === 2 ? 'No recent report' : 'Reported source', reported_at: now, name: this.FIXTURE_ACTION, status: index === 2 ? 'stale' : 'completed', status_label: index === 2 ? 'Last reported running' : 'completed', running: false, reported_status: index === 2 ? 'running' : null,
        session_id: agent.session.id, session_name: agent.session.name, task_id: agent.task.id, task_name: agent.task.name,
        source: this.FIXTURE_SOURCE, started_label: '2026-10-01 09:00:00 UTC', finished_label: index === 2 ? null : '2026-10-01 09:00:01 UTC',
        duration_label: index === 2 ? null : '1.00 s', duration_note: index === 2 ? null : 'Measured by the reporting caller', elapsed_label: null,
        exit_code: index === 2 ? null : 0, captured_at: now, role_label: 'Agent'}]}));
    const history = activity[0].records[0];
    activity[0].records = [
      {...history, id: 'synthetic-running-one', status: 'running', status_label: 'running', running: true, finished_label: null, duration_label: null, duration_note: null, exit_code: null, elapsed_label: '3.0 s elapsed · since hook receipt'},
      {...history, id: 'synthetic-running-two', status: 'running', status_label: 'running', running: true, finished_label: null, duration_label: null, duration_note: null, exit_code: null, elapsed_label: '4.0 s elapsed · since hook receipt'},
      ...Array.from({length: 18}, /** @param {unknown} _ @param {number} index @returns {OfflineAction} */ (_, index) => ({...history, id: 'synthetic-completed-' + index}))
    ];
    const links = [this.commandLink(logical_agents[0], this.LINKED_ROOT, this.LINKED_PROCESSES, now), this.commandLink(logical_agents[1], this.SECOND_LINKED_ROOT, this.SECOND_LINKED_PROCESSES, now)];
    return {sampled_at: now, started_at: now - this.HISTORY_SECONDS, groups, logical_agents, activity, totals,
      command_observations: links.map(/** @param {OfflineCommandLink} link @returns {OfflineCommandObservation} */ link => link[0]),
      command_bindings: links.map(/** @param {OfflineCommandLink} link @returns {OfflineCommandBinding} */ link => link[1]),
      errors: [], limitations: ['Synthetic QA data; no real process or logical agent observations.']};
  }

  /** @param {OfflineLogical} agent @param {number} root @param {number} count @param {number} now @returns {OfflineCommandLink} */
  static commandLink(agent, root, count, now) {
    /** @type {OfflineCommandOwner} */
    const owner = {agent_id: agent.id, session_id: agent.session.id, task_id: agent.task.id, execution_id: agent.executions[0].id};
    const processes = Array.from({length: count}, /** @param {unknown} _ @param {number} offset @returns {OfflineCommandProcess} */ (_, offset) => ({pid: root + offset,
      parent: offset === 0 ? 1 : offset < 4 ? root : root + 1, started: (this.BIRTH_SECONDS + root + offset) + ':0',
      name: offset ? 'fixture-child-' + offset : 'fixture-command-' + agent.family.toLowerCase(),
      cpu_percent: 2.5 + offset, resident_bytes: (20 + offset) * this.MEBIBYTE, received_per_second: offset ? null : 1024,
      sent_per_second: offset ? null : 512, disk_read_per_second: 1024, disk_write_per_second: 2048}));
    const binding = 'offline-binding-' + root;
    const trust = {scope: 'reported-command-process', provenance: 'caller-reported', validation: 'os-identity-matched'};
    const tree_history = Array.from({length: this.HISTORY_POINTS}, /** @param {unknown} _ @param {number} index @returns {OfflineTreeSample} */ (_, index) => ({
      time: now - this.HISTORY_SECONDS + index * this.SAMPLE_SECONDS,
      processes: index % this.GAP_EVERY === 0 ? [] : processes.map(/** @param {OfflineCommandProcess} process @returns {OfflineCommandProcess} */ process => ({...process, cpu_percent: 10 + Math.sin(index / 4) * 5}))}));
    const linked = now - this.HISTORY_SECONDS - 10;
    return [{binding_id: binding, generation: 1, owner, pid: root, started: processes[0].started, sampled_at: now, ...trust,
      coverage: 'Synthetic local process only; network partial', metrics: processes[0], descendants: processes.slice(1),
      totals: {cpu_seconds: 1.5, resident_bytes: 20 * this.MEBIBYTE, peak_resident_bytes: 30 * this.MEBIBYTE, received_bytes: 0, sent_bytes: 0, disk_read_bytes: 0, disk_written_bytes: 0, samples: 1},
      history: [], tree_history},
      {id: binding, run_id: 'offline-run-' + root, generation: 1, owner, identity: {pid: root, started: processes[0].started, user: 501, executable: '/synthetic/offline-command'},
        received_at: linked, verified_at: linked, lease_until: now + 3600, ...trust}];
  }

  /** @param {import("playwright").Browser} browser @returns {Promise<void>} */
  static async check(browser) {
    const page = await browser.newPage({viewport: this.NORMAL, serviceWorkers: 'block'});
    page.on('pageerror', /** @param {Error} error @returns {number} */ error => this.report.errors.push(error.message));
    await page.route('**/*', /** @param {import('playwright').Route} route @returns {Promise<void>} */ route => this.route(route));
    await page.exposeFunction('offlineRustOverview', /** @param {OfflineRequest} request @returns {Overview} */ request => OverviewFixtureClient.project(request));
    await page.addInitScript(/** @param {{fixture: OfflineFixture, seconds: number, step: number}} input @returns {void} */ input => {
      const fixture = input.fixture;
      const boundary = /** @type {OfflineBrowser} */ (globalThis);
      boundary.offlineFixture = fixture;
      /** @type {Map<number, string[]>} */
      const transfers = new Map();
      let samples = 0;
      /** @param {number} now @returns {void} */
      const refresh = now => {
        fixture.sampled_at = now;
        for (const observation of fixture.command_observations) {
          observation.sampled_at = now;
          observation.tree_history.forEach(/** @param {OfflineTreeSample} sample @param {number} index @returns {void} */ (sample, index) => {
            sample.time = now - input.seconds + index * input.step;
          });
        }
      };
      boundary.offlineNative = {
        open: /** @param {NativeSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => {
          fixture.totals.samples = ++samples;
          const now = Date.now() / 1000;
          refresh(now);
          const view = await boundary.offlineRustOverview({...fixture, now, selection});
          const text = JSON.stringify(view);
          const chunks = text.match(/[\s\S]{1,8000}/g);
          if (!chunks) throw new Error('The synthetic Overview transfer cannot be empty.');
          transfers.set(samples, chunks);
          return {id: samples, pages: chunks.length, bytes: new TextEncoder().encode(text).length, error: ''};
        },
        page: /** @param {number} id @param {number} index @returns {Promise<SnapshotPage>} */ async (id, index) => {
          const chunks = transfers.get(id);
          if (!chunks) throw new Error('The synthetic Overview ticket must exist.');
          return {id, index, content: chunks[index], error: ''};
        },
        release: /** @param {number} id @returns {Promise<boolean>} */ async id => transfers.delete(id)
      };
      /** @param {string} capability @param {string} payload @returns {Promise<string>} */
      const hostCall = async (capability, payload) => {
        if (capability === 'window.title') return JSON.stringify({value: null});
        /** @type {OfflineHostRequest} */
        const request = JSON.parse(payload);
        if (capability !== 'companion.invoke' || request.service !== 'snapshotReader') return JSON.stringify({ok: false, error: 'The synthetic offline host only answers snapshotReader.'});
        if (request.op === 'init') return JSON.stringify({value: {fields: [], methods: ['open', 'page', 'release'], listeners: []}});
        const member = /** @type {((...values: unknown[]) => Promise<unknown>) | undefined} */ (Reflect.get(boundary.offlineNative, request.name));
        if (request.op === 'call' && member) return JSON.stringify({value: await member(...request.args)});
        return JSON.stringify({value: null});
      };
      Object.defineProperty(boundary, '__tachyonNativeHostCall', {value: hostCall, configurable: true});
      Object.defineProperty(boundary, '__tachyonWindowControls', {value: Object.freeze(['title']), configurable: true});
    }, {fixture: this.fixture(), seconds: this.HISTORY_SECONDS, step: this.SAMPLE_SECONDS});
    await page.goto(this.ORIGIN, {waitUntil: 'load'});
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelectorAll('.session-card').length === 4 && document.querySelector('.main-footer')?.textContent?.includes('samples') === true, undefined, {polling: this.POLL_MILLISECONDS});
    await page.evaluate(/** @returns {void} */ () => {
      const label = document.createElement('div');
      label.textContent = 'OFFLINE UI QA · SYNTHETIC DATA · NOT A NATIVE CAPTURE';
      label.style.cssText = 'position:fixed;bottom:2px;left:4px;z-index:99999;font:10px monospace;color:#ffdfaa;background:#121722;padding:3px;pointer-events:none';
      document.body.append(label);
    });
    await this.shell(page);
    await this.grid(page, 3);
    await this.runningCards(page);
    await this.summary(page);
    await this.capture(page, 'running-1240');
    await page.setViewportSize({width: 1920, height: 928});
    await this.grid(page, 4);
    await this.capture(page, 'running-1920');
    await page.setViewportSize(this.NORMAL);
    await this.openSession(page, this.LINKED_SESSION);
    await this.panelColumns(page, 1);
    await this.tabs(page);
    await this.charts(page, 'CPU', '% of one logical core');
    await this.capture(page, 'resources-1240');
    await this.charts(page, 'Network I/O', 'KiB / s (partial)');
    await this.charts(page, 'Memory', 'MiB RSS');
    await this.charts(page, 'Disk I/O', 'KiB / s (storage)');
    await this.sessionUsage(page);
    await this.tab(page, this.TREE_TAB);
    await this.graphShell(page);
    await this.tree(page);
    await this.processSelection(page);
    await this.capture(page, 'tree-default-1240');
    await this.zoomScale(page);
    await this.zoom(page);
    await this.capture(page, 'tree-1240');
    await this.tabKeyboard(page);
    await this.listContinuity(page);
    await this.tab(page, this.ACTIVITY_TAB);
    await this.activity(page);
    await this.capture(page, 'activity-1240');
    await this.unlinkedSession(page);
    await this.automaticCollection(page);
    await page.setViewportSize(this.WIDE);
    await this.closeWithEscape(page);
    await this.shell(page);
    await this.grid(page, 4);
    await this.capture(page, 'running-1680');
    await this.openSession(page, this.LINKED_SESSION);
    await this.panelColumns(page, 2);
    await this.tab(page, this.TREE_TAB);
    await this.graphShell(page);
    await this.capture(page, 'tree-1680');
    await page.setViewportSize(this.SMALL);
    await this.smallWindow(page);
    await page.setViewportSize({width: 900, height: 700});
    await this.narrowWindow(page);
    this.report.checks.push('Session cards three columns 1240, four 1680/1920, two 1000/900; open panel beside the cards at 1240/1680/1000 and full width with Back at 900; one session tree; no document overflow');
    assert.deepEqual(this.report.errors, [], 'Offline UI acceptance failures must be fixed before a passing report.');
    await page.close();
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async runningCards(page) {
    assert.equal(await page.locator('[data-agent="offline-agent-4"],[data-agent="offline-agent-5"]').count(), 0, 'Completed and unconfirmed reports are excluded from current session cards.');
    assert.doesNotMatch(await page.locator('.session-area').innerText(), /\bstale\b/i);
    const sessions = await page.locator(this.SESSION_CARD).evaluateAll(/** @param {Element[]} cards @returns {string[]} */ cards => cards.map(/** @param {Element} card @returns {string} */ card => (/** @type {HTMLElement} */ (card)).dataset.sessionId ?? ''));
    assert.deepEqual(sessions.sort(), ['offline-test-session-0', 'offline-test-session-1', 'offline-test-session-2', 'offline-test-session-3'], 'Each card carries its exact reported session ID.');
    assert.equal(await page.locator('.agent-row').count(), 4, 'One compact row per active agent.');
    assert.ok((await page.locator('.agent-row').evaluateAll(/** @param {Element[]} rows @returns {(string|undefined)[]} */ rows => rows.map(/** @param {Element} row @returns {string|undefined} */ row => (/** @type {HTMLElement} */ (row)).dataset.agent))).every(Boolean));
    assert.ok((await page.locator('.agent-row .agent-role').allTextContents()).every(/** @param {string} text @returns {boolean} */ text => text === 'Agent'));
    assert.ok((await page.locator('.agent-row .agent-tool-name').allTextContents()).every(/** @param {string} text @returns {boolean} */ text => text === this.FIXTURE_ACTION));
    assert.doesNotMatch(await page.locator('.session-grid').innerText(), /elapsed/, 'Missing execution start cannot become a timer.');
    assert.equal(await page.locator('.session-card .graph-node,.session-card .history-card,.session-card .action-history-row').count(), 0, 'Cards stay compact; details belong to the session panel.');
    assert.doesNotMatch(await page.locator('.session-area').innerText(), /Execution timing unavailable|No running action reported|full command unavailable/);
    this.report.checks.push('Running cards: exact session IDs, one Agent row per running agent, completed and recovered reports excluded, no fabricated timer');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async summary(page) {
    /** @type {Record<string, string>} */
    const expected = {agents: '4', sessions: '4', tools: '4', harnesses: '2'};
    for (const [key, value] of Object.entries(expected)) {
      assert.equal(await page.locator('.metric-card[data-summary="' + key + '"] strong').innerText(), value, 'Summary ' + key + ' counts current synthetic work.');
    }
    assert.match(await page.locator('.metric-card[data-summary="harnesses"] p').innerText(), /Claude Code, Codex/);
    this.report.checks.push({summary: expected});
  }

  /** @param {import('playwright').Page} page @param {string} key @returns {Promise<void>} */
  static async openSession(page, key) {
    await page.locator('.session-card-open[data-session="' + key + '"]').click();
    await page.waitForFunction(/** @param {string} key @returns {boolean} */ key => {
      const panel = /** @type {HTMLElement | null} */ (document.querySelector('.session-panel.is-open'));
      return panel?.dataset.session === key && ['detail', 'inactive'].includes(panel.dataset.sessionView ?? '');
    }, key);
    assert.equal(await page.locator('.session-card.is-selected').count(), 1);
    assert.equal(await page.locator('.session-card.is-selected').getAttribute('data-session'), key);
    assert.equal(await page.locator('.session-card-open[data-session="' + key + '"]').getAttribute('aria-expanded'), 'true');
  }

  /** @param {import('playwright').Page} page @param {string} name @returns {Promise<void>} */
  static async tab(page, name) {
    await page.locator('[data-session-tab="' + name + '"]').click();
    await page.waitForFunction(/** @param {string} name @returns {boolean} */ name => document.querySelector('[data-session-tab="' + name + '"]')?.getAttribute('aria-selected') === 'true'
      && document.querySelector('#session-panel-' + name) !== null, name);
  }

  /** @param {import('playwright').Page} page @returns {Promise<string | null>} */
  static async selectedTab(page) {
    return page.locator(this.SELECTED_TAB).getAttribute('data-session-tab');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async tabs(page) {
    assert.deepEqual(await page.locator(this.SESSION_TAB).allTextContents(), ['Resources', 'Process tree', 'Activity']);
    assert.equal(await this.selectedTab(page), this.RESOURCES_TAB, 'A fresh session panel opens on Resources.');
    assert.equal(await page.locator('[role="tabpanel"]').count(), 1, 'Only the selected tab panel is rendered.');
    const tabs = await page.locator(this.SESSION_TAB).evaluateAll(/** @param {Element[]} elements @returns {OfflineTab[]} */ elements => elements.map(/** @param {Element} tab @returns {OfflineTab} */ tab => {
      const box = tab.getBoundingClientRect();
      const range = document.createRange(); range.selectNodeContents(tab);
      const text = [...range.getClientRects()].map(/** @param {DOMRect} rect @returns {OfflineTextBounds} */ rect => ({top:rect.top,bottom:rect.bottom,left:rect.left,right:rect.right}));
      return {label:tab.textContent,height:box.height,top:box.top,bottom:box.bottom,left:box.left,right:box.right,appearance:getComputedStyle(tab).appearance,text};
    }));
    this.report.checks.push({sessionTabGeometry: tabs});
    assert.ok(tabs.every(/** @param {OfflineTab} tab @returns {boolean} */ tab => tab.height >= this.MINIMUM_TAB_HEIGHT && tab.appearance === 'none'), 'Session tabs have usable sizing and custom appearance: ' + JSON.stringify(tabs));
    assert.ok(tabs.every(/** @param {OfflineTab} tab @returns {boolean} */ tab => tab.text.every(/** @param {OfflineTextBounds} rect @returns {boolean} */ rect => rect.top >= tab.top && rect.bottom <= tab.bottom && rect.left >= tab.left && rect.right <= tab.right)), 'Each session-tab label stays inside its own button.');
  }

  /** @param {import('playwright').Page} page @param {number} columns @returns {Promise<void>} */
  static async panelColumns(page, columns) {
    const layout = await this.panelLayout(page);
    assert.ok(layout.listVisible, 'The session cards stay visible beside the open panel.');
    assert.ok(layout.panel.x >= layout.list.x + layout.list.width - 1, 'The session panel is a column beside the cards, not an overlay: ' + JSON.stringify(layout));
    assert.ok(Math.abs(layout.list.width - layout.panel.width) < 40, 'Cards and panel each take about half the width: ' + JSON.stringify(layout));
    const cards = await this.rectangles(page, this.SESSION_CARD);
    const lefts = new Set(cards.map(/** @param {OfflineRectangle} card @returns {number} */ card => Math.round(card.x)));
    if (lefts.size !== columns) this.report.errors.push('Open panel: expected ' + columns + ' card column(s), got ' + JSON.stringify(cards));
    this.report.checks.push({panelLayout: layout, columns, cards});
  }

  /** @param {import('playwright').Page} page @returns {Promise<PanelLayout>} */
  static async panelLayout(page) {
    return page.evaluate(/** @returns {PanelLayout} */ () => {
      const list = document.querySelector('.session-area');
      const panel = document.querySelector('.session-panel');
      const back = document.querySelector('.session-panel-back');
      if (!list || !panel || !back) throw new Error('The session list, panel and Back control must exist.');
      /** @param {Element} element @returns {OfflineRectangle} */
      const box = element => { const r = element.getBoundingClientRect(); return {x: r.x, y: r.y, width: r.width, height: r.height}; };
      return {list: box(list), panel: box(panel), listVisible: getComputedStyle(list).display !== 'none', backVisible: getComputedStyle(back).display !== 'none'};
    });
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async sessionUsage(page) {
    const heading = await page.locator('.history-card h3').innerText();
    assert.match(heading, /7 current processes · 1 verified roots/, 'Session usage counts the verified command root and its six descendants only, never the shared runtimes.');
    assert.equal(await page.locator('.history-card').count(), 1, 'One session usage chart card.');
    this.report.checks.push({sessionUsage: heading, scope: await page.locator('#session-panel-resources').getAttribute('data-scope')});
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async processSelection(page) {
    const child = page.locator(this.GRAPH_NODE + '[data-pid="' + this.SELECTED_CHILD + '"]');
    assert.match(await child.innerText(), /fixture-child-1/);
    await child.click();
    await page.waitForFunction(/** @param {string} text @returns {boolean} */ text => document.querySelector('.session-selected-process')?.textContent?.includes(text) === true, 'PID ' + this.SELECTED_CHILD);
    await this.polls(page, 2);
    assert.match(await page.locator(this.SELECTED_PROCESS).innerText(), new RegExp('PID ' + this.SELECTED_CHILD));
    await this.tab(page, this.RESOURCES_TAB);
    await this.tab(page, this.TREE_TAB);
    await page.waitForFunction(/** @param {{pid: number, count: number}} input @returns {boolean} */ input => document.querySelectorAll('.graph-node').length === input.count && document.querySelector('.session-selected-process')?.textContent?.includes('PID ' + input.pid) === true, {pid: this.SELECTED_CHILD, count: this.LINKED_PROCESSES});
    assert.equal(await page.locator(this.GRAPH_NODE + '.selected').getAttribute('data-pid'), String(this.SELECTED_CHILD), 'The picked tree node stays highlighted.');
    this.report.checks.push('Synthetic native boundary: process pick in the session tree survives two polls and a Resources/Process tree tab switch');
  }

  /** @param {import('playwright').Page} page @param {string} key @param {string} name @returns {Promise<void>} */
  static async pressTab(page, key, name) {
    await page.keyboard.press(key);
    await page.waitForFunction(/** @param {string} name @returns {boolean} */ name => document.activeElement?.getAttribute('data-session-tab') === name && document.activeElement.getAttribute('aria-selected') === 'true', name);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async tabKeyboard(page) {
    await page.locator('[data-session-tab="' + this.TREE_TAB + '"]').focus();
    await this.pressTab(page, 'ArrowRight', this.ACTIVITY_TAB);
    await this.pressTab(page, 'Home', this.RESOURCES_TAB);
    await this.pressTab(page, 'End', this.ACTIVITY_TAB);
    await this.pressTab(page, 'ArrowLeft', this.TREE_TAB);
    this.report.checks.push('Session tabs: ArrowRight, Home, End and ArrowLeft move focus and selection');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async listContinuity(page) {
    const viewport = {width: 1240, height: 450};
    await page.setViewportSize(viewport);
    const list = page.locator(this.SESSION_LIST);
    const bounds = await list.boundingBox();
    assert.ok(bounds);
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.wheel(0, 130);
    const before = (await this.settledScroll(page, this.SESSION_LIST))[0];
    assert.ok(before > 0, 'Actual wheel input scrolls the session list.');
    await page.locator('[data-session-tab="' + this.TREE_TAB + '"]').focus();
    await this.polls(page, 3);
    /** @type {ListContinuity} */
    const continuity = {before, after: await list.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop), viewport,
      selection: await page.locator('.session-card.is-selected').getAttribute('data-session'),
      focused: await page.evaluate(/** @param {string} name @returns {boolean} */ name => document.activeElement?.getAttribute('data-session-tab') === name, this.TREE_TAB)};
    this.report.checks.push({listContinuity: continuity});
    this.requireListContinuity(continuity);
    await this.capture(page, 'list-scroll-1240x450');
    await page.setViewportSize(this.NORMAL);
  }

  /** @param {ListContinuity} continuity @returns {void} */
  static requireListContinuity(continuity) {
    if (continuity.after !== continuity.before) this.report.errors.push('Session list scroll reset across three polls: ' + JSON.stringify(continuity));
    if (continuity.selection !== this.LINKED_SESSION) this.report.errors.push('Session selection lost across polling.');
    if (!continuity.focused) this.report.errors.push('Session tab focus lost across polling.');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async activity(page) {
    const rows = page.locator(this.ACTION_ROW);
    assert.equal(await rows.count(), 20, 'The selected session shows its bounded twenty-action history.');
    const completed = page.locator(this.ACTION_ROW + '[data-action="synthetic-completed-0"]');
    assert.match(await completed.innerText(), /fixture-action/);
    assert.match(await completed.innerText(), /1\.00 s/);
    assert.match(await completed.innerText(), /synthetic fixture only/);
    assert.equal(await page.locator(this.ACTION_ROW + ':not([data-session="offline-test-session-0"])').count(), 0, 'Another session activity never leaks into this panel.');
    assert.equal(await page.locator(this.ELAPSED).count(), 2, 'Only the two synthetic live records have elapsed labels.');
    const live = page.locator('[data-action="synthetic-running-one"]');
    assert.match(await live.innerText(), /3\.0 s elapsed/);
    assert.match(await page.locator('[data-action="synthetic-running-two"]').innerText(), /4\.0 s elapsed/);
    const scroller = page.locator('.activity-scroll');
    const scrollBounds = await scroller.boundingBox();
    assert.ok(scrollBounds);
    await page.mouse.move(scrollBounds.x + scrollBounds.width / 2, scrollBounds.y + scrollBounds.height / 2);
    await page.mouse.wheel(0, 190);
    const actionOffset = (await this.settledScroll(page, '.activity-scroll'))[0];
    assert.ok(actionOffset > 0, 'Actual wheel input scrolls the bounded activity panel.');
    await this.polls(page, 2);
    assert.ok(Math.abs(await scroller.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop) - actionOffset) < 2, 'Session activity scroll survives real UI refreshes.');
    await page.evaluate(/** @returns {OfflineAction} */ () => Object.assign((/** @type {OfflineBrowser} */ (globalThis)).offlineFixture.activity[0].records[0], {
      status: 'completed', status_label: 'completed', running: false, elapsed_label: null, finished_label: '2026-10-01 09:00:05 UTC',
      duration_label: '5.00 s', duration_note: 'Measured by the reporting caller', exit_code: 0
    }));
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('[data-action="synthetic-running-one"] .status-pill')?.textContent === 'completed');
    assert.equal(await live.count(), 1, 'A completion updates the same action row without duplicating it.');
    assert.equal(await live.locator(this.ELAPSED).count(), 0);
    assert.match(await live.innerText(), /5\.00 s/);
    assert.equal(await page.locator(this.ELAPSED).count(), 1, 'The concurrent unfinished action stays running.');
    this.report.checks.push('Synthetic boundary: independent concurrent action labels; same-ID completion; session activity scroll preserved across two polls');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async unlinkedSession(page) {
    await this.openSession(page, this.UNLINKED_SESSION);
    assert.equal(await this.selectedTab(page), this.ACTIVITY_TAB, 'Switching sessions keeps the current tab.');
    const record = page.locator(this.ACTION_ROW);
    assert.equal(await record.count(), 1);
    assert.equal(await record.locator('.action-history-times').innerText(), 'Started · hook receipt\n2026-10-01 09:00:00 UTC');
    assert.match(await record.innerText(), /Last reported running/);
    assert.doesNotMatch(await page.locator('#session-panel-activity').innerText(), /\bstale\b/i);
    await this.tab(page, this.RESOURCES_TAB);
    assert.match(await page.locator('#session-panel-resources .empty').innerText(), /No measured session usage\./, 'An unlinked session never borrows shared runtime totals.');
    assert.equal(await page.locator(this.RESOURCE_CHART).count(), 0);
    await this.tab(page, this.TREE_TAB);
    assert.equal(await page.locator(this.GRAPH_NODE).count(), 0, 'Shared synthetic runtimes are never drawn as this session tree.');
    this.report.checks.push('Unlinked session: honest empty Resources and Process tree; shared runtimes not attributed; tab kept across session switch');
    await this.openSession(page, this.LINKED_SESSION);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async closeWithEscape(page) {
    await page.locator('.session-panel').focus();
    await page.keyboard.press('Escape');
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-panel.is-open') === null && document.querySelector('.session-card.is-selected') === null);
    await this.requireCardFocus(page);
    this.report.checks.push('Escape closes the session panel and returns focus to the originating card');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async requireCardFocus(page) {
    await page.waitForFunction(/** @param {string} key @returns {boolean} */ key => (/** @type {HTMLElement | null} */ (document.activeElement))?.dataset.session === key && document.activeElement?.classList.contains('session-card-open') === true, this.LINKED_SESSION);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async smallWindow(page) {
    await this.shell(page);
    await this.panelColumns(page, 1);
    await this.graphShell(page);
    await this.capture(page, 'tree-1000');
    await this.closeWithEscape(page);
    await this.grid(page, 2);
    await this.capture(page, 'running-1000');
    await this.openSession(page, this.LINKED_SESSION);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async narrowWindow(page) {
    await this.shell(page);
    const layout = await this.panelLayout(page);
    assert.equal(layout.listVisible, false, 'Below 920 px the panel takes the whole content width.');
    assert.ok(layout.backVisible, 'Below 920 px the panel shows Back to all sessions.');
    this.report.checks.push({narrowPanel: layout});
    await this.capture(page, 'panel-900');
    await page.locator('.session-panel-back').click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-panel.is-open') === null);
    await this.requireCardFocus(page);
    await this.shell(page);
    await this.grid(page, 2);
    await this.capture(page, 'running-900');
  }

  /** @param {import('playwright').Page} page @param {string} action @param {number} count @returns {Promise<void>} */
  static async zoomSteps(page, action, count) {
    for (let index=0;index<count;index++) {
      const previous = await page.locator(this.ZOOM_VALUE).innerText();
      await page.locator('[data-zoom="' + action + '"]').click();
      await page.waitForFunction(/** @param {string} previous @returns {boolean} */ previous => document.querySelector('.graph-zoom-value')?.textContent!==previous,previous);
    }
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async shell(page) {
    const result = await page.evaluate(/** @returns {OfflineShell} */ () => {
      const footer = document.querySelector('.main-footer');
      if (!footer) throw new Error('The app footer is missing.');
      return {width: innerWidth, height: innerHeight, documentX: document.documentElement.scrollWidth > innerWidth,
        documentY: document.documentElement.scrollHeight > innerHeight, footerBottom: footer.getBoundingClientRect().bottom,
        summaryCards: document.querySelectorAll('.metric-card').length};
    });
    assert.equal(result.documentX, false, 'No whole-page sideways overflow.');
    assert.equal(result.documentY, false, 'No whole-page vertical overflow.');
    assert.ok(result.footerBottom <= result.height + 1, 'The collection footer stays inside the window.');
    assert.equal(result.summaryCards, 4);
    this.report.checks.push({shell: result});
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<OfflineRectangle[]>} */
  static async rectangles(page, selector) {
    await page.evaluate(/** @returns {Promise<number>} */ () => new Promise(/** @param {(time: number | PromiseLike<number>) => void} resolve @returns {number} */ resolve => requestAnimationFrame(/** @param {number} _time @returns {number} */ _time => requestAnimationFrame(resolve))));
    return page.locator(selector).evaluateAll(/** @param {Element[]} elements @returns {OfflineRectangle[]} */ elements => elements.map(/** @param {Element} element @returns {OfflineRectangle} */ element => {
      const r = element.getBoundingClientRect();
      return {x: r.x, y: r.y, width: r.width, height: r.height};
    }));
  }

  /** @param {import("playwright").Page} page @param {number} columns @returns {Promise<void>} */
  static async grid(page, columns) {
    await page.waitForFunction(/** @param {string} selector @returns {boolean} */ selector => document.querySelectorAll(selector).length === 4, this.SESSION_CARD);
    const cards = await this.rectangles(page, this.SESSION_CARD);
    assert.equal(cards.length, 4, 'Session cards show ALL actual fixture sessions.');
    assert.ok(cards.every(/** @param {OfflineRectangle} card @returns {boolean} */ card => Math.abs(card.width - cards[0].width) < 1), 'Session cards have equal columns.');
    const firstRow = cards.filter(/** @param {OfflineRectangle} card @returns {boolean} */ card => Math.abs(card.y - cards[0].y) < 1);
    const lefts = new Set(firstRow.map(/** @param {OfflineRectangle} card @returns {number} */ card => Math.round(card.x)));
    if (firstRow.length !== columns || lefts.size !== columns) this.report.errors.push('Session cards: expected ' + columns + ' column(s), got ' + JSON.stringify(cards));
    this.report.checks.push({grid: this.SESSION_CARD, columns, cards});
  }

  /** @param {import("playwright").Page} page @returns {Promise<number>} */
  static async samples(page) {
    const matched = (await page.locator(this.FOOTER_SAMPLES).innerText()).match(/\d+/);
    assert.ok(matched, 'The actual footer reports a sample count.');
    return Number(matched[0]);
  }

  /** @param {import("playwright").Page} page @param {number} count @returns {Promise<void>} */
  static async polls(page, count) {
    const before = await this.samples(page);
    try {
      await page.waitForFunction(/** @param {number} target @returns {boolean} */ target => {
        const matched = document.querySelector('.main-footer > span:last-child')?.textContent?.match(/\d+/);
        if (!matched) throw new Error('The actual footer sample count is missing.');
        return Number(matched[0]) >= target;
      }, before + count, {timeout: 12000});
    } catch (error) {
      this.report.errors.push('Collection stalled waiting for ' + count + ' polls after sample ' + before + ': ' + await this.collectionState(page));
      throw error;
    }
  }

  /** @param {import("playwright").Page} page @returns {Promise<string>} */
  static async collectionState(page) {
    return page.evaluate(/** @returns {string} */ () => JSON.stringify({footer: document.querySelector('.main-footer')?.textContent ?? null,
      live: document.querySelector('.live-state')?.textContent ?? null, alerts: [...document.querySelectorAll('w-alert')].map(/** @param {Element} alert @returns {string | null} */ alert => alert.textContent)}));
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async tree(page) {
    const tree = await page.locator(this.GRAPH_GROUP).evaluate(/** @param {Element} group @returns {OfflineTree} */ group => {
      /** @type {Map<string, number>} */
      const pids = new Map();
      const nodes = [...group.querySelectorAll('.graph-node')].map(/** @param {Element} node @returns {OfflineNode} */ node => {
        const element = /** @type {HTMLElement} */ (node);
        pids.set(element.dataset.identity ?? '', Number(element.dataset.pid));
        return {pid: Number(element.dataset.pid), identity: element.dataset.identity ?? '', x: Number(element.parentElement?.getAttribute('x')), y: Number(element.parentElement?.getAttribute('y')), text: element.textContent};
      });
      const drawn = /** @param {Element} path @returns {boolean} */ path => Boolean(path.getAttribute('d'));
      const edges = [...group.querySelectorAll('.child-branch')].filter(drawn).map(/** @param {Element} edge @returns {number[]} */ edge => {
        const element = /** @type {SVGElement} */ (edge);
        return [pids.get(element.dataset.parentIdentity ?? '') ?? 0, pids.get(element.dataset.childIdentity ?? '') ?? 0];
      });
      return {nodes, edges, trunks: [...group.querySelectorAll('.shared-trunk')].filter(drawn).length};
    });
    const rootPid = this.LINKED_ROOT;
    const root = tree.nodes.find(/** @param {OfflineNode} node @returns {boolean} */ node => node.pid === rootPid);
    assert.ok(root);
    const childPids = [rootPid + 1, rootPid + 2, rootPid + 3];
    const children = tree.nodes.filter(/** @param {OfflineNode} node @returns {boolean} */ node => childPids.includes(node.pid));
    assert.equal(tree.nodes.length, this.LINKED_PROCESSES);
    assert.ok(children.every(/** @param {OfflineNode} node @returns {boolean} */ node => node.y > root.y), 'Each actual child appears below its parent; sibling rows may wrap to the viewport.');
    assert.equal(new Set(children.map(/** @param {OfflineNode} node @returns {string} */ node => node.x + ':' + node.y)).size, 3, 'Children use separate cards.');
    assert.deepEqual(tree.edges.sort(), [[rootPid, rootPid + 1], [rootPid, rootPid + 2], [rootPid, rootPid + 3], [rootPid + 1, rootPid + 4], [rootPid + 1, rootPid + 5], [rootPid + 1, rootPid + 6]].sort());
    assert.equal(tree.trunks, 2, 'One shared trunk per actual parent with children.');
    assert.ok(tree.nodes.every(/** @param {OfflineNode} node @returns {boolean} */ node => /CPU/.test(node.text ?? '') && /RSS/.test(node.text ?? '')), 'Individual readable metric cards.');
    this.report.checks.push({tree});
  }

  /** @param {import('playwright').Page} page @returns {Promise<OfflineGraphGeometry>} */
  static async geometry(page) {
    return page.locator(this.GRAPH_GROUP).evaluate(/** @param {Element} group @returns {OfflineGraphGeometry} */ group => {
      const canvas = group.querySelector('.graph-canvas');
      if (!canvas) throw new Error('The active graph canvas is missing.');
      return {zoom: group.querySelector('.graph-zoom-value')?.textContent ?? null,
        canvasWidth: Number(canvas.getAttribute('width')), viewWidth: Number(canvas.getAttribute('viewBox')?.split(' ')[2]),
        nodes: [...group.querySelectorAll('.graph-node')].map(/** @param {Element} node @returns {Omit<OfflineNode,'text'>} */ node => {const element=/** @type {HTMLElement} */ (node);return {pid:Number(element.dataset.pid),identity:element.dataset.identity ?? '',x:Number(element.parentElement?.getAttribute('x')),y:Number(element.parentElement?.getAttribute('y'))};}),
        edges: [...group.querySelectorAll('.child-branch')].map(/** @param {Element} edge @returns {(string|undefined)[]} */ edge => {const element=/** @type {SVGElement} */ (edge);return [element.dataset.parentIdentity,element.dataset.childIdentity];})};
    });
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async zoomScale(page) {
    const before = await this.geometry(page);
    assert.equal(before.zoom,'100%','Fresh session trees begin at readable normal card size.');
    await this.zoomSteps(page, 'out', 6);
    const after = await this.geometry(page);
    assert.equal(after.zoom, '40%');
    assert.ok(Math.abs(after.canvasWidth - after.viewWidth * 0.4) < 2, 'Zoom scales the drawn canvas against its world viewBox: ' + JSON.stringify(after));
    assert.deepEqual(after.nodes, before.nodes, 'Zoom keeps actual world coordinates; it does not move cards.');
    assert.deepEqual(after.edges, before.edges, 'Zoom preserves all actual parent relationships.');
    this.report.checks.push({treeZoomScale:{before,after,initialReadableScale:true}});
    await this.capture(page,'tree-zoom-out-1240');
  }

  /** @param {import('playwright').Page} page @returns {Promise<string>} */
  static async zoomValue(page) {
    return page.locator(this.ZOOM_VALUE).innerText();
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<number[]>} */
  static async settledScroll(page, selector) {
    await page.waitForFunction(/** @param {string} selector @returns {Promise<boolean>} */ async selector => {
      const element = document.querySelector(selector);
      if (!element) throw new Error('The scrolled element is missing: ' + selector);
      const first = [element.scrollTop, element.scrollLeft].join(':');
      await new Promise(/** @param {(value: unknown) => void} resolve @returns {ReturnType<typeof setTimeout>} */ resolve => setTimeout(resolve, 250));
      return first === [element.scrollTop, element.scrollLeft].join(':');
    }, selector, {polling: this.POLL_MILLISECONDS});
    return page.locator(selector).evaluate(/** @param {Element} element @returns {number[]} */ element => [element.scrollTop, element.scrollLeft]);
  }

  /** @param {import('playwright').Page} page @returns {Promise<number[]>} */
  static async pan(page) {
    return page.locator(this.GRAPH_SCROLL).evaluate(/** @param {Element} element @returns {number[]} */ element => [element.scrollTop, element.scrollLeft]);
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async zoom(page) {
    await this.zoomSteps(page, 'in', 7);
    const zoomed = await this.zoomValue(page);
    assert.equal(zoomed, '110%');
    const bounds = await page.locator(this.GRAPH_SCROLL).boundingBox();
    assert.ok(bounds);
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.wheel(160, 120);
    const scroll = await this.settledScroll(page, this.GRAPH_SCROLL);
    assert.ok(scroll[0] > 0 || scroll[1] > 0, 'Actual wheel input pans the zoomed tree.');
    await this.polls(page, 3);
    assert.equal(await this.zoomValue(page), zoomed);
    const after = await this.pan(page);
    assert.deepEqual(after, scroll, 'Session tree pan survives live polls.');
    await this.openSession(page, this.SECOND_LINKED_SESSION);
    await page.waitForFunction(/** @param {number} count @returns {boolean} */ count => document.querySelectorAll('.graph-node').length === count, this.SECOND_LINKED_PROCESSES);
    assert.equal(await this.selectedTab(page), this.TREE_TAB, 'Switching sessions keeps the Process tree tab.');
    assert.equal(await this.zoomValue(page), '100%', 'Another session tree starts at its own zoom.');
    await this.zoomSteps(page, 'out', 1);
    assert.equal(await this.zoomValue(page), '90%');
    await this.openSession(page, this.LINKED_SESSION);
    await page.waitForFunction(/** @param {number} count @returns {boolean} */ count => document.querySelectorAll('.graph-node').length === count, this.LINKED_PROCESSES);
    assert.equal(await this.zoomValue(page), zoomed, 'Another session zoom never changes this one.');
    await page.locator('[data-zoom="fit"]').click();
    await page.waitForTimeout(200);
    const fit = await page.locator(this.GRAPH_SCROLL).evaluate(/** @param {Element} element @returns {OfflineFit} */ element => {
      const canvas = element.querySelector('.graph-canvas');
      if (!canvas) throw new Error('The active graph canvas is missing.');
      return {width:Number(canvas.getAttribute('width')),height:Number(canvas.getAttribute('height')),availableWidth:element.clientWidth,availableHeight:element.clientHeight};
    });
    assert.ok(fit.width <= fit.availableWidth && fit.height <= fit.availableHeight, 'Explicit Fit includes the complete tree: ' + JSON.stringify(fit));
    const fitted = await this.zoomValue(page);
    await page.setViewportSize(this.SMALL);
    await page.waitForTimeout(200);
    assert.equal(await this.zoomValue(page), fitted, 'Resize preserves manually chosen Fit/zoom.');
    await page.setViewportSize(this.NORMAL);
    await page.waitForTimeout(200);
    this.report.checks.push({treePan:{before:scroll,after},sessionZoom:{zoomed,fit,fitted,otherSession:'90%',manualResizePreserved:true}});
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async graphShell(page) {
    await page.waitForFunction(/** @param {number} count @returns {boolean} */ count => document.querySelectorAll('.graph-group').length === 1 && document.querySelectorAll('.graph-node').length === count, this.LINKED_PROCESSES, {timeout: 6000});
    await page.evaluate(/** @returns {Promise<number>} */ () => new Promise(/** @param {(time: number | PromiseLike<number>) => void} resolve @returns {number} */ resolve => requestAnimationFrame(/** @param {number} _time @returns {number} */ _time => requestAnimationFrame(resolve))));
    assert.equal(await page.locator(this.GRAPH_GROUP).count(),1,'Only the selected session tree is rendered.');
    assert.equal(await page.locator(this.GRAPH_GROUP).getAttribute('data-group'), this.LINKED_SESSION);
    const layout = await page.locator('#session-panel-tree').evaluate(/** @param {Element} panel @returns {OfflineTreeArea} */ panel => {
      const group=panel.querySelector('.graph-group');const scroll=panel.querySelector('.group-graph-scroll');if(!group||!scroll)throw new Error('The session tree is missing.');
      const outer=panel.getBoundingClientRect();const inner=scroll.getBoundingClientRect();
      return {width:outer.width,bottom:outer.bottom,groupWidth:group.getBoundingClientRect().width,scrollWidth:inner.width,scrollHeight:inner.height,scrollBottom:inner.bottom};
    });
    this.report.checks.push({sessionTreeArea:layout});
    assert.ok(layout.groupWidth <= layout.width + 1 && layout.scrollWidth >= layout.groupWidth - 4 && layout.scrollHeight > 0 && layout.scrollBottom <= layout.bottom + 1, 'The session tree spans its panel width and stays inside the tab panel: '+JSON.stringify(layout));
  }

  /** @param {import("playwright").Page} page @param {string} name @param {string} unit @returns {Promise<void>} */
  static async charts(page, name, unit) {
    await page.locator('.chart-tabs').getByText(name, {exact: true}).click();
    await page.waitForFunction(/** @param {string} unit @returns {boolean} */ unit => {
      const canvas = document.querySelector('.resource-chart');
      return (/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(canvas)?.options.scales.y.title.text === unit;
    }, unit);
    const charts = await page.locator(this.RESOURCE_CHART).evaluateAll(/** @param {Element[]} canvases @returns {OfflineChartGeometry[]} */ canvases => canvases.map(/** @param {Element} canvas @returns {OfflineChartGeometry} */ canvas => {
      const chart = (/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(canvas);
      if(!chart)throw new Error('The resource chart is missing.');
      return {width: canvas.clientWidth, height: canvas.clientHeight, unit: chart.options.scales.y.title.text,
        window: chart.options.scales.x.max - chart.options.scales.x.min,
        title: chart.options.scales.x.title.text, gaps: chart.data.datasets.some(/** @param {import('../../app/client/models/ChartDataset.ts').ChartDataset} series @returns {boolean} */ series => series.data.some(/** @param {import('../../app/client/models/ChartPoint.ts').ChartPoint} point @returns {boolean} */ point => point.y === null)),
        points: chart.data.datasets[0].data.length, spanGaps: chart.data.datasets[0].spanGaps};
    }));
    this.report.checks.push({charts: name, values: charts});
    assert.equal(charts.length, 1, 'One session usage chart.');
    assert.ok(charts.every(/** @param {OfflineChartGeometry} chart @returns {boolean} */ chart => chart.unit === unit && chart.window === this.HISTORY_SECONDS && chart.points === this.HISTORY_POINTS && chart.height >= 180), name + ' chart geometry: ' + JSON.stringify(charts));
    assert.ok(charts.every(/** @param {OfflineChartGeometry} chart @returns {boolean} */ chart => chart.gaps && chart.spanGaps === false), name + ' keeps missing samples as gaps.');
    const canvas = page.locator(this.RESOURCE_CHART).first();
    const rect = await canvas.boundingBox();
    assert.ok(rect);
    await page.mouse.move(rect.x + rect.width * 0.8, rect.y + rect.height * 0.4);
    await page.waitForFunction(/** @returns {boolean} */ () => ((/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(document.querySelector('.resource-chart'))?.tooltip.opacity ?? 0) > 0);
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async automaticCollection(page) {
    assert.equal(await page.locator('.topbar w-btn').count(),0,'Automatic collection needs no Pause/Refresh controls.');
    await this.polls(page,3);
    const before = await this.samples(page);
    await page.evaluate(/** @returns {void} */ () => {
      const boundary=/** @type {OfflineBrowser} */ (globalThis);
      const original = boundary.offlineNative.open;
      let pendingFailure = true;
      boundary.offlineNative.open = /** @param {NativeSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => {
        if (pendingFailure) {
          pendingFailure = false;
          return {id:0,pages:0,bytes:0,error:'Synthetic offline collector interruption'};
        }
        return original(selection);
      };
    });
    const handle = await page.waitForFunction(/** @returns {OfflineDisconnected | null} */ () => {
      if (document.querySelector('.live-state.state-Disconnected') === null) return null;
      return {alerts: [...document.querySelectorAll('w-alert.alert')].map(/** @param {Element} alert @returns {string} */ alert => alert.textContent ?? ''),
        values: [...document.querySelectorAll('.metric-card strong')].map(/** @param {Element} value @returns {string} */ value => value.textContent ?? ''),
        footer: document.querySelector('.main-footer > span:last-child')?.textContent ?? ''};
    },undefined,{timeout:6000, polling: this.POLL_MILLISECONDS});
    const disconnected = await handle.jsonValue();
    assert.ok(disconnected, 'The disconnected state was captured.');
    this.report.checks.push({disconnected});
    assert.ok(disconnected.alerts.some(/** @param {string} text @returns {boolean} */ text => /Local measurements unavailable\. Retrying automatically/.test(text)), 'A failed collection shows the retry warning.');
    assert.ok(disconnected.values.every(/** @param {string} text @returns {boolean} */ text => text === '—'), 'Disconnected summary cards show a dash, never old numbers.');
    assert.equal(Number(disconnected.footer.match(/\d+/)?.[0]), before, 'A failed collection retains last measured sample count.');
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.live-state.state-Live') !== null && document.querySelector('w-alert.alert') === null,undefined,{timeout:6000});
    await this.polls(page,1);
    this.report.checks.push('Automatic launch/cadence collection across three polls; failure shows Disconnected with dashes and keeps the last sample; next poll recovers without manual controls');
  }

  /** @param {import("playwright").Page} page @param {string} label @returns {Promise<void>} */
  static async capture(page, label) {
    await page.evaluate(/** @returns {void} */ () => {
      document.querySelector('[data-offline-qa-watermark]')?.remove();
      const marker = document.createElement('div');
      marker.dataset.offlineQaWatermark = 'true';
      marker.textContent = 'OFFLINE QA · SYNTHETIC DATA · NOT NATIVE';
      marker.style.cssText = 'position:fixed;bottom:2px;right:4px;z-index:2147483647;font:11px monospace;color:#ffdfaa;background:#121722;padding:4px;pointer-events:none';
      document.body.append(marker);
    });
    const clipping = await page.evaluate(/** @returns {OfflineClipping[]} */ () => {
      const selector = '.session-card-heading > p,.agent-role,.history-card h3,.graph-name,.graph-identity,.graph-measurement,.action-history-heading strong,.session-tab';
      return [...document.querySelectorAll(selector)].flatMap(/** @param {Element} element @returns {OfflineClipping[]} */ element => {
        const card = element.closest('.session-card,.history-card,.graph-node,.action-history-row,.session-tab');
        if (!card) return [];
        const bounds = card.getBoundingClientRect();
        if (!bounds || bounds.width === 0 || bounds.height === 0) return [];
        const range = document.createRange();
        range.selectNodeContents(element);
        const rects = [...range.getClientRects()];
        const outside = rects.some(/** @param {DOMRect} rect @returns {boolean} */ rect => rect.width > 0 && (rect.left < bounds.left - 1 || rect.right > bounds.right + 1 || rect.top < bounds.top - 1 || rect.bottom > bounds.bottom + 1));
        return outside ? [{text: element.textContent, card: card.className}] : [];
      });
    });
    this.report.checks.push({readability: label, clippedEssentialText: clipping});
    if (clipping.length) this.report.errors.push('Essential card text crosses card bounds: ' + label + ': ' + JSON.stringify(clipping));
    await page.screenshot({path: QaPathUtils.evidence('offline-ui-' + label + '.png').pathname});
  }
}

await OfflineUiTest.run();
