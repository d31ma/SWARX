import {QaPathUtils} from './QaPathUtils.mjs';
import { OverviewFixtureClient } from './OverviewFixtureClient.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import {BrowserBundleProvenanceUtils} from './utils/BrowserBundleProvenanceUtils.mjs';
const {chromium} = await import(QaPathUtils.file('OBSERVATORY_PLAYWRIGHT_MODULE').href);

/** @typedef {{x: number, y: number, width: number, height: number}} OfflineRectangle */
/** @typedef {{width: number, height: number, documentX: boolean, documentY: boolean, sidebar: boolean, nav: {text: string | null, bottom: number}[]}} OfflineShell */
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
/** @typedef {{sampled_at:number,started_at:number,groups:OfflineGroup[],logical_agents:OfflineLogical[],activity:OfflineActivity[],totals:OfflineTotals,errors:string[],limitations:string[]}} OfflineFixture */
/** @typedef {OfflineFixture & {now:number,selection:NativeSelection}} OfflineRequest */
/** @typedef {{open:(selection:NativeSelection)=>Promise<SnapshotTransfer>,page:(id:number,index:number)=>Promise<SnapshotPage>,release:(id:number)=>Promise<boolean>}} OfflineNative */
/** @typedef {{op:string,service:string,name:string,args:unknown[]}} OfflineHostRequest */
/** @typedef {{options:{scales:{x:{max:number,min:number,title:{text:string}},y:{title:{text:string}}}},data:{datasets:import('../../app/client/models/ChartDataset.ts').ChartDataset[]},tooltip:{opacity:number}}} OfflineChart */
/** @typedef {{getChart:(canvas:Element|null)=>OfflineChart|undefined}} OfflineChartLibrary */
/** @typedef {typeof globalThis & {offlineFixture:OfflineFixture,offlineNative:OfflineNative,offlineRustOverview:(request:OfflineRequest)=>Promise<Overview>,Chart?:OfflineChartLibrary}} OfflineBrowser */
/** @typedef {{pid:number,x:number,y:number,text:string|null}} OfflineNode */
/** @typedef {{nodes:OfflineNode[],edges:number[][],trunks:number}} OfflineTree */
/** @typedef {{zoom:string|null,nodes:Omit<OfflineNode,'text'>[],edges:(string|undefined)[][]}} OfflineGraphGeometry */
/** @typedef {{unit:string,window:number,points:number,height:number,width:number,title:string,gaps:boolean,spanGaps:boolean}} OfflineChartGeometry */
/** @typedef {{top:number,bottom:number,left:number,right:number}} OfflineTextBounds */
/** @typedef {OfflineTextBounds & {label:string|null,height:number,lineHeight:string,appearance:string,text:OfflineTextBounds[]}} OfflineTab */
/** @typedef {{before:number,after:number,viewport:{width:number,height:number},selection:string,focused:boolean}} TableContinuity */

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
    const totals = {cpu_seconds: 3.2, resident_bytes: 200 * 1048576, peak_resident_bytes: 250 * 1048576,
      received_bytes: 10240, sent_bytes: 2048, disk_read_bytes: 0, disk_written_bytes: 0, samples: 1};
    const groups = [0, 1, 2, 3].map(/** @param {number} index @returns {OfflineGroup} */ index => {
      const pid = 90000 + index * 100;
      const family = index % 2 ? 'Claude' : 'Codex';
      const processes = [0, 1, 2, 3, 4, 5, 6].map(/** @param {number} offset @returns {OfflineProcess} */ offset => ({pid: pid + offset, parent: offset === 0 ? 1 : offset < 4 ? pid : pid + 1,
        started: 'OFFLINE-TEST-START-' + (pid + offset), name: offset ? 'fixture-child-' + offset : 'fixture-' + family,
        cpu_percent: 2.5 + offset, resident_bytes: (20 + offset) * 1048576, received_per_second: null, sent_per_second: null,
        disk_read_per_second: 1024, disk_write_per_second: 2048}));
      return {id: 'offline-' + pid + '-start', pid, started: 'OFFLINE-TEST-START-' + pid, family,
        name: family + ' synthetic runtime ' + (index + 1), status: 'process-present', scope: 'shared-runtime', shared: true,
        instrumented: false, identity_evidence: 'synthetic test boundary', members: processes.map(/** @param {OfflineProcess} process @returns {number} */ process => process.pid), processes,
        task_status: 'unavailable', metrics: {cpu_percent: 35.5, resident_bytes: 200 * 1048576,
          cpu_covered_processes: 7, memory_covered_processes: 7, disk_covered_processes: 7,
          received_per_second: index === 1 ? null : 1024, sent_per_second: index === 1 ? null : 512,
          disk_read_per_second: 10240, disk_write_per_second: 2048,
          network_status: index === 1 ? 'unavailable' : 'partial', network_covered_processes: index === 1 ? 0 : 1, totals},
        history: Array.from({length: 61}, /** @param {unknown} _ @param {number} i @returns {(number|null)[]} */ (_, i) => [now - 120 + i * 2, i % 13 === 0 ? null : 30 + Math.sin(i / 4) * 20,
          (200 + Math.sin(i / 8) * 12) * 1048576, index === 1 || i % 13 === 0 ? null : 1024 * (1 + i % 5),
          index === 1 ? null : 512, i % 13 === 0 ? null : 10240, 2048])};
    });
    const logical_agents = [0, 1, 2, 3, 4, 5].map(/** @param {number} index @returns {OfflineLogical} */ index => ({id: 'offline-agent-' + index,
      name: 'Synthetic QA agent ' + (index + 1), family: index % 2 ? 'Claude' : 'Codex',
      session: {id: 'offline-test-session-' + index, name: 'Offline test session', source: 'synthetic test boundary'},
      status: index === 4 ? 'completed' : 'running', source: 'synthetic fixture only', captured_at: now,
      feed_mode: index === 5 ? 'recovered' : 'events',
      task: {id: 'offline-task-' + index, name: 'Synthetic UI acceptance task', status: index === 4 ? 'completed' : 'running', depends_on: []},
      executions: [{id: 'offline-action-' + index, name: 'fixture-action', status: index === 4 ? 'completed' : 'running', started_at: 0,
        session_id: 'offline-test-session-' + index, task_id: 'offline-task-' + index, source: 'synthetic fixture only',
        exit_code: index === 4 ? 0 : null, duration_seconds: null}],
      age_seconds: index === 5 ? 240 : 0, stale: index === 5}));
    const activity = logical_agents.slice(0, 4).map(/** @param {OfflineLogical} agent @param {number} index @returns {OfflineActivity} */ (agent, index) => ({id: agent.id + ':' + agent.session.id, name: agent.name, family: agent.family, captured_at: now,
      session_id: agent.session.id, session_name: agent.session.name,
      records: [{id: 'offline-history-' + index, agent_id: agent.id, agent_name: agent.name, agent_family: agent.family, freshness: index === 2 ? 'No recent report' : 'Reported source', reported_at: now, name: 'fixture-action', status: index === 2 ? 'stale' : 'completed', status_label: index === 2 ? 'Last reported running' : 'completed', running: false, reported_status: index === 2 ? 'running' : null,
        session_id: agent.session.id, session_name: agent.session.name, task_id: agent.task.id, task_name: agent.task.name,
        source: 'synthetic fixture only', started_label: '2026-10-01 09:00:00 UTC', finished_label: index === 2 ? null : '2026-10-01 09:00:01 UTC',
        duration_label: index === 2 ? null : '1.00 s', duration_note: index === 2 ? null : 'Measured by the reporting caller', elapsed_label: null,
        exit_code: index === 2 ? null : 0, captured_at: now}]}));
    const history = activity[0].records[0];
    activity[0].records = [
      {...history, id: 'synthetic-running-one', status: 'running', status_label: 'running', running: true, finished_label: null, duration_label: null, duration_note: null, exit_code: null, elapsed_label: '3.0 s elapsed · since hook receipt'},
      {...history, id: 'synthetic-running-two', status: 'running', status_label: 'running', running: true, finished_label: null, duration_label: null, duration_note: null, exit_code: null, elapsed_label: '4.0 s elapsed · since hook receipt'},
      ...Array.from({length: 18}, /** @param {unknown} _ @param {number} index @returns {OfflineAction} */ (_, index) => ({...history, id: 'synthetic-completed-' + index}))
    ];
    return {sampled_at: now, started_at: now - 120, groups, logical_agents, activity, totals,
      errors: [], limitations: ['Synthetic QA data; no real process or logical agent observations.']};
  }

  /** @param {import("playwright").Browser} browser @returns {Promise<void>} */
  static async check(browser) {
    const page = await browser.newPage({viewport: {width: 1240, height: 850}, serviceWorkers: 'block'});
    page.on('pageerror', /** @param {Error} error @returns {number} */ error => this.report.errors.push(error.message));
    await page.route('**/*', /** @param {import('playwright').Route} route @returns {Promise<void>} */ route => this.route(route));
    await page.exposeFunction('offlineRustOverview', /** @param {OfflineRequest} request @returns {Overview} */ request => OverviewFixtureClient.project(request));
    await page.addInitScript(/** @param {OfflineFixture} fixture @returns {void} */ fixture => {
      const boundary = /** @type {OfflineBrowser} */ (globalThis);
      boundary.offlineFixture = fixture;
      /** @type {Map<number, string[]>} */
      const transfers = new Map();
      let samples = 0;
      boundary.offlineNative = {
        open: /** @param {NativeSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => {
          fixture.totals.samples = ++samples;
          fixture.sampled_at = Date.now() / 1000;
          const view = await boundary.offlineRustOverview({...fixture, now: Date.now() / 1000, selection});
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
        /** @type {OfflineHostRequest} */
        const request = JSON.parse(payload);
        if (capability !== 'companion.invoke' || request.service !== 'snapshotReader') return JSON.stringify({ok: false, error: 'The synthetic offline host only answers snapshotReader.'});
        if (request.op === 'init') return JSON.stringify({value: {fields: [], methods: ['open', 'page', 'release'], listeners: []}});
        const member = /** @type {((...values: unknown[]) => Promise<unknown>) | undefined} */ (Reflect.get(boundary.offlineNative, request.name));
        if (request.op === 'call' && member) return JSON.stringify({value: await member(...request.args)});
        return JSON.stringify({value: null});
      };
      Object.defineProperty(boundary, '__tachyonNativeHostCall', {value: hostCall, configurable: true});
    }, this.fixture());
    await page.goto(this.ORIGIN, {waitUntil: 'load'});
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelectorAll('.logical-agent-row').length === 4 && document.querySelector('.main-footer')?.textContent?.includes('samples') === true);
    await page.evaluate(/** @returns {void} */ () => {
      const label = document.createElement('div');
      label.textContent = 'OFFLINE UI QA · SYNTHETIC DATA · NOT A NATIVE CAPTURE';
      label.style.cssText = 'position:fixed;bottom:2px;left:4px;z-index:99999;font:10px monospace;color:#ffdfaa;background:#121722;padding:3px;pointer-events:none';
      document.body.append(label);
    });
    await this.shell(page);
    await this.grid(page, '.logical-agent-row', 2);
    assert.equal(await page.locator('[data-agent="offline-agent-4"],[data-agent="offline-agent-5"]').count(), 0, 'Completed and unconfirmed reports are excluded from current running cards.');
    assert.doesNotMatch(await page.locator('.logical-panel').innerText(), /\bstale\b/i);
    assert.equal(await page.locator('.metric-card').filter({hasText: 'RECOVERED RECORDS'}).locator('strong').innerText(), '1');
    assert.equal(await page.locator('.logical-agent-row .agent-resources,.logical-agent-row .execution-history').count(), 0, 'Lower resource/history area is removed.');
    assert.match(await page.locator('.current-action').first().innerText(), /fixture-action/);
    assert.equal(await page.locator('.action-timing').count(), 0, 'Missing execution start cannot become a timer.');
    assert.ok((await page.locator('.agent-session-id').allTextContents()).every(/** @param {string} text @returns {boolean} */ text => text.startsWith('Session ID · ')));
    assert.ok((await page.locator('.logical-agent-row').evaluateAll(/** @param {Element[]} rows @returns {(string|undefined)[]} */ rows => rows.map(/** @param {Element} row @returns {string|undefined} */ row => (/** @type {HTMLElement} */ (row)).dataset.agent))).every(Boolean));
    assert.doesNotMatch(await page.locator('.logical-panel').innerText(), /Execution timing unavailable|No running action reported|full command unavailable/);
    await this.capture(page, 'running-1240');
    await page.setViewportSize({width:1920,height:928});
    await this.capture(page, 'running-1920');
    await page.setViewportSize({width:1240,height:850});
    await this.section(page, 'Processes', 'processes');
    await this.grid(page, '.table-branch', 2);
    await this.tableProcesses(page);
    await page.locator('.session-row').nth(1).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-row.selected')?.textContent?.includes('runtime 3') === true);
    await page.setViewportSize({width:1240,height:450});
    const input = page.getByRole('textbox', {name: 'Filter processes by provider, name or PID'});
    await input.focus();
    const tableScroll = page.locator('.session-list');
    const tableBounds = await tableScroll.boundingBox();
    assert.ok(tableBounds);
    await page.mouse.move(tableBounds.x + tableBounds.width / 2, tableBounds.y + tableBounds.height / 2);
    await page.mouse.wheel(0, 130);
    await page.waitForTimeout(150);
    const before = await tableScroll.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop);
    assert.ok(before > 0, 'Actual wheel input scrolls the long Table.');
    await this.polls(page, 3);
    const continuity = {before, after: await tableScroll.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop),
      viewport: {width:1240,height:450},
      selection: await page.locator('.session-row.selected').innerText(),
      focused: await input.evaluate(/** @param {Element} element @returns {boolean} */ element => document.activeElement === element)};
    this.report.checks.push({tableContinuity: continuity});
    this.requireTableContinuity(continuity);
    await this.capture(page, 'table-scroll-1240x450');
    await page.setViewportSize({width:1240,height:850});
    await this.capture(page, 'table-1240');
    await page.getByText('Tree', {exact: true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.graph-pane')?.classList.contains('mode-tree') === true && document.querySelectorAll('.graph-group').length === 1);
    await this.graphShell(page);
    await this.tree(page);
    await this.capture(page, 'tree-default-1240');
    await this.adaptive(page);
    await this.zoom(page);
    await this.capture(page, 'tree-1240');
    await this.section(page, 'Resource history', 'history');
    await this.grid(page, '.history-card', 2);
    await this.charts(page, 'CPU', '% of one logical core', true);
    await this.capture(page, 'history-1240');
    await this.charts(page, 'Network', 'KiB / s (partial)', true);
    await this.charts(page, 'Memory', 'MiB RSS', false);
    await this.charts(page, 'Disk I/O', 'KiB / s (storage)', true);
    await this.section(page, 'Runtimes', 'runtime');
    await this.grid(page, '.runtime-card', 2);
    await this.section(page, 'Activity', 'activity');
    await this.grid(page, '.activity-card', 2);
    assert.match(await page.locator('.activity-card').first().innerText(), /fixture-action/);
    assert.match(await page.locator('.activity-card').first().innerText(), /1\.00 s/);
    assert.match(await page.locator('.activity-card').first().innerText(), /synthetic fixture only/);
    assert.doesNotMatch(await page.locator('.activity-overview').innerText(), /\bstale\b/i);
    assert.equal(await page.locator('.activity-card').nth(2).locator('.action-history-times').innerText(), 'Started · hook receipt\n2026-10-01 09:00:00 UTC');
    assert.equal(await page.locator('.action-elapsed').count(), 2, 'Only the two synthetic live records have elapsed labels.');
    const live = page.locator('[data-action="synthetic-running-one"]');
    assert.match(await live.innerText(), /3\.0 s elapsed/);
    assert.match(await page.locator('[data-action="synthetic-running-two"]').innerText(), /4\.0 s elapsed/);
    const actionList = page.locator('.action-history-list').first();
    const actionBounds = await actionList.boundingBox();
    assert.ok(actionBounds);
    await page.mouse.move(actionBounds.x + actionBounds.width / 2, actionBounds.y + actionBounds.height / 2);
    await page.mouse.wheel(0, 190);
    await page.waitForTimeout(150);
    const actionOffset = await actionList.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop);
    assert.ok(actionOffset > 0, 'Actual wheel input scrolls the bounded action list.');
    await this.polls(page, 2);
    assert.ok(Math.abs(await actionList.evaluate(/** @param {Element} element @returns {number} */ element => element.scrollTop) - actionOffset) < 2, 'Per-agent history scroll survives real UI refreshes.');
    await page.evaluate(/** @returns {OfflineAction} */ () => Object.assign((/** @type {OfflineBrowser} */ (globalThis)).offlineFixture.activity[0].records[0], {
      status: 'completed', status_label: 'completed', running: false, elapsed_label: null, finished_label: '2026-10-01 09:00:05 UTC',
      duration_label: '5.00 s', duration_note: 'Measured by the reporting caller', exit_code: 0
    }));
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('[data-action="synthetic-running-one"] .status-pill')?.textContent === 'completed');
    assert.equal(await live.count(), 1, 'A completion updates the same action row without duplicating it.');
    assert.equal(await live.locator('.action-elapsed').count(), 0);
    assert.match(await live.innerText(), /5\.00 s/);
    assert.equal(await page.locator('.action-elapsed').count(), 1, 'The concurrent unfinished action stays running.');
    this.report.checks.push('Synthetic boundary: independent concurrent action labels; same-ID completion; bounded per-card scroll preserved across two polls');
    await this.capture(page, 'activity-1240');
    await this.automaticCollection(page);
    await page.setViewportSize({width: 1680, height: 1000});
    await this.section(page, 'Processes', 'processes');
    await page.getByText('Table', {exact: true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-list')?.classList.contains('mode-table') === true);
    await this.shell(page);
    await this.grid(page, '.table-branch', 4);
    await this.capture(page, 'table-1680');
    await page.getByText('Tree', {exact: true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.graph-pane')?.classList.contains('mode-tree') === true);
    await this.graphShell(page);
    await this.capture(page, 'tree-1680');
    await this.section(page, "What's running", 'running');
    await this.grid(page, '.logical-agent-row', 4);
    await this.capture(page, 'running-1680');
    await this.section(page, 'Activity', 'activity');
    await this.grid(page, '.activity-card', 4);
    await this.capture(page, 'activity-1680');
    await page.setViewportSize({width: 1000, height: 700});
    await this.smallPages(page);
    await this.section(page, 'Processes', 'processes');
    await page.getByText('Table', {exact: true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-list')?.classList.contains('mode-table') === true);
    await this.grid(page, '.table-branch', 1);
    await page.setViewportSize({width: 900, height: 700});
    await this.shell(page);
    await this.grid(page, '.table-branch', 1);
    await this.capture(page, 'table-900');
    this.report.checks.push('All five pages at1240x850/1000x700; Table/Agents four columns1680, two1240, one1000/900; one full-area Tree; no document/sidebar overflow');
    assert.deepEqual(this.report.errors, [], 'Offline UI acceptance failures must be fixed before a passing report.');
    await page.close();
  }

  /** @param {import("playwright").Page} page @param {string} name @param {string} section @returns {Promise<void>} */
  static async section(page, name, section) {
    await page.locator('.nav-item').filter({hasText: name}).click();
    await page.waitForFunction(/** @param {string} section @returns {boolean} */ section => {
      const main = document.querySelector('main');
      if (!main) throw new Error('The app main panel is missing.');
      return main.classList.contains('section-' + section);
    }, section);
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async tableProcesses(page) {
    assert.equal(await page.locator('.table-branch').count(), 4);
    assert.equal(await page.locator('.table-branch .tree-expansion,.table-branch .tree-node,.table-branch .process-table-rows').count(), 0, 'Table contains runtime cards only; descendants belong to Tree.');
    const card = page.locator('.table-branch[data-group="offline-90000-start"]');
    await card.locator('.session-row').click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.table-branch[data-group="offline-90000-start"] .session-row.selected') !== null);
    await this.polls(page, 2);
    assert.equal(await card.locator('.session-row.selected').count(), 1, 'Runtime selection survives polling.');
    await page.getByText('Tree', {exact:true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelectorAll('.graph-node').length === 7);
    const child = page.locator('.graph-node[data-pid="90001"]');
    assert.match(await child.innerText(), /fixture-child-1/);
    await child.click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.process-measurement-note')?.textContent?.includes('Selected PID 90001') === true);
    await this.polls(page, 2);
    assert.match(await page.locator('.process-measurement-note').innerText(), /Selected PID 90001/);
    await page.getByText('Table', {exact:true}).click();
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('.session-list')?.classList.contains('mode-table') === true);
    assert.match(await page.locator('.process-measurement-note').innerText(), /Selected PID 90001/);
    assert.equal(await page.locator('.table-branch .tree-expansion,.table-branch .tree-node').count(), 0);
    this.report.checks.push('Synthetic native boundary: Table cards only; actual descendant selection in dedicated Tree; shared PID selection survives two polls and view switches');
  }

  /** @param {TableContinuity} continuity @returns {void} */
  static requireTableContinuity(continuity) {
    if (continuity.after !== continuity.before) this.report.errors.push('Table scroll reset across three polls: ' + JSON.stringify(continuity));
    if (!continuity.selection.includes('runtime 3')) this.report.errors.push('Table selection lost across polling.');
    if (!continuity.focused) this.report.errors.push('Table filter focus lost across polling.');
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async smallPages(page) {
    for (const [name, section, selector] of [["What's running", 'running', '.logical-agent-row'], ['Processes', 'processes', '.graph-group'],
      ['Resource history', 'history', '.history-card'], ['Runtimes', 'runtime', '.runtime-card'], ['Activity', 'activity', '.activity-card']]) {
      await this.section(page, name, section);
      await this.shell(page);
      if (section === 'processes') await this.graphShell(page);
      else await this.grid(page, selector, 1);
      await this.capture(page, section + '-1000');
    }
  }

  /** @param {import('playwright').Page} page @param {number} count @returns {Promise<void>} */
  static async zoomOut(page, count) {
    for (let index=0;index<count;index++) {
      const previous = await page.locator('.graph-zoom-value').innerText();
      await page.locator('[data-zoom="out"]').click();
      await page.waitForFunction(/** @param {string} previous @returns {boolean} */ previous => document.querySelector('.graph-zoom-value')?.textContent!==previous,previous);
    }
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async shell(page) {
    const result = await page.evaluate(/** @returns {OfflineShell} */ () => {
      const sidebar = document.querySelector('.sidebar');
      if (!sidebar) throw new Error('The app sidebar is missing.');
      return {width: innerWidth, height: innerHeight, documentX: document.documentElement.scrollWidth > innerWidth,
      documentY: document.documentElement.scrollHeight > innerHeight, sidebar: sidebar.scrollHeight > sidebar.clientHeight,
      nav: [...document.querySelectorAll('.nav-item')].map(/** @param {Element} element @returns {{text: string | null, bottom: number}} */ element => ({text: element.textContent, bottom: element.getBoundingClientRect().bottom}))};
    });
    assert.equal(result.documentX, false, 'No whole-page sideways overflow.');
    assert.equal(result.documentY, false, 'No whole-page vertical overflow.');
    assert.equal(result.sidebar, false, 'All five sidebar entries fit initially.');
    assert.equal(result.nav.length, 5);
    assert.ok(result.nav.every(/** @param {{text: string | null, bottom: number}} item @returns {boolean} */ item => item.bottom <= result.height));
    this.report.checks.push({shell: result});
  }

  /** @param {import("playwright").Page} page @param {string} selector @param {number} columns @returns {Promise<void>} */
  static async grid(page, selector, columns) {
    await page.waitForFunction(/** @param {string} selector @returns {boolean} */ selector => document.querySelectorAll(selector).length === 4, selector);
    await page.evaluate(/** @returns {Promise<number>} */ () => new Promise(/** @param {(time: number | PromiseLike<number>) => void} resolve @returns {number} */ resolve => requestAnimationFrame(/** @param {number} _time @returns {number} */ _time => requestAnimationFrame(resolve))));
    const cards = await page.locator(selector).evaluateAll(/** @param {Element[]} elements @returns {OfflineRectangle[]} */ elements => elements.map(/** @param {Element} element @returns {OfflineRectangle} */ element => {
      const r = element.getBoundingClientRect();
      return {x: r.x, y: r.y, width: r.width, height: r.height};
    }));
    assert.equal(cards.length, 4, selector + ' shows ALL actual fixture items.');
    assert.ok(Math.abs(cards[0].width - cards[1].width) < 1, selector + ' equal columns');
    const layout = columns === 1 ? cards[0].x === cards[1].x && cards[1].y > cards[0].y
      : cards.slice(0, columns).every(/** @param {OfflineRectangle} card @returns {boolean} */ card => card.y === cards[0].y && Math.abs(card.width - cards[0].width) < 1) && cards[1].x > cards[0].x;
    if (!layout) this.report.errors.push(selector + ': expected ' + columns + ' column(s), got ' + JSON.stringify(cards.slice(0, 2)));
    this.report.checks.push({grid: selector, columns, cards});
  }

  /** @param {import("playwright").Page} page @returns {Promise<number>} */
  static async samples(page) {
    const matched = (await page.locator('.main-footer > span:last-child').innerText()).match(/\d+/);
    assert.ok(matched, 'The actual footer reports a sample count.');
    return Number(matched[0]);
  }

  /** @param {import("playwright").Page} page @param {number} count @returns {Promise<void>} */
  static async polls(page, count) {
    const before = await this.samples(page);
    await page.waitForFunction(/** @param {number} target @returns {boolean} */ target => {
      const matched = document.querySelector('.main-footer > span:last-child')?.textContent?.match(/\d+/);
      if (!matched) throw new Error('The actual footer sample count is missing.');
      return Number(matched[0]) >= target;
    }, before + count, {timeout: 12000});
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async tree(page) {
    const tree = await page.locator('.graph-group').first().evaluate(/** @param {Element} group @returns {OfflineTree} */ group => ({
      nodes: [...group.querySelectorAll('.graph-node')].map(/** @param {Element} node @returns {OfflineNode} */ node => {const element=/** @type {HTMLElement} */ (node);return {pid:Number(element.dataset.pid),x:Number(element.parentElement?.getAttribute('x')),y:Number(element.parentElement?.getAttribute('y')),text:element.textContent};}),
      edges: [...group.querySelectorAll('.child-branch')].map(/** @param {Element} edge @returns {number[]} */ edge => {const element=/** @type {SVGElement} */ (edge);return [Number(element.dataset.parent),Number(element.dataset.child)];}),
      trunks: group.querySelectorAll('.shared-trunk').length
    }));
    const root = tree.nodes.find(/** @param {OfflineNode} node @returns {boolean} */ node => node.pid === 90000);
    assert.ok(root);
    const children = tree.nodes.filter(/** @param {OfflineNode} node @returns {boolean} */ node => [90001, 90002, 90003].includes(node.pid));
    assert.equal(tree.nodes.length, 7);
    assert.ok(children.every(/** @param {OfflineNode} node @returns {boolean} */ node => node.y > root.y));
    assert.ok(children.every(/** @param {OfflineNode} node @returns {boolean} */ node => node.y > root.y), 'Each actual child appears below its parent; sibling rows may wrap to the viewport.');
    assert.equal(new Set(children.map(/** @param {OfflineNode} node @returns {number} */ node => node.x)).size, 3, 'Children use separate horizontal cards.');
    assert.deepEqual(tree.edges.sort(), [[90000,90001],[90000,90002],[90000,90003],[90001,90004],[90001,90005],[90001,90006]].sort());
    assert.equal(tree.trunks, 2, 'One shared trunk per actual parent with children.');
    assert.ok(tree.nodes.every(/** @param {OfflineNode} node @returns {boolean} */ node => /CPU|cpu/i.test(node.text ?? '') && /RSS|Memory/i.test(node.text ?? '')), 'Individual readable metric cards.');
    this.report.checks.push({tree});
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async adaptive(page) {
    const geometry = /** @returns {Promise<OfflineGraphGeometry>} */ () => page.locator('.graph-group').evaluate(/** @param {Element} group @returns {OfflineGraphGeometry} */ group => ({
      zoom: group.querySelector('.graph-zoom-value')?.textContent ?? null,
      nodes: [...group.querySelectorAll('.graph-node')].map(/** @param {Element} node @returns {Omit<OfflineNode,'text'>} */ node => {const element=/** @type {HTMLElement} */ (node);return {pid:Number(element.dataset.pid),x:Number(element.parentElement?.getAttribute('x')),y:Number(element.parentElement?.getAttribute('y'))};}),
      edges: [...group.querySelectorAll('.child-branch')].map(/** @param {Element} edge @returns {(string|undefined)[]} */ edge => {const element=/** @type {SVGElement} */ (edge);return [element.dataset.parent,element.dataset.child];})
    }));
    const before = await geometry();
    assert.equal(before.zoom,'100%','Fresh groups begin at readable normal card size.');
    await this.zoomOut(page, 6);
    const after = await geometry();
    assert.notDeepEqual(after.nodes,before.nodes,'Zoom reflows actual world coordinates, not merely a fixed canvas transform.');
    assert.deepEqual(after.edges,before.edges,'Reflow preserves all actual parent relationships.');
    const siblingRows = /** @param {Omit<OfflineNode,'text'>[]} nodes @returns {number} */ nodes => new Set(nodes.filter(/** @param {Omit<OfflineNode,'text'>} node @returns {boolean} */ node => [90001,90002,90003].includes(node.pid)).map(/** @param {Omit<OfflineNode,'text'>} node @returns {number} */ node => node.y)).size;
    assert.ok(siblingRows(after.nodes)<siblingRows(before.nodes),'Zoom out packs actual siblings into fewer rows.');
    this.report.checks.push({adaptiveTreeReflow:{before,after,initialReadableScale:true}});
    await this.capture(page,'tree-adaptive-reflow-1240');
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async zoom(page) {
    const first = await page.locator('.graph-group-tabs [role="tab"]').nth(0).getAttribute('data-graph-group');
    const second = await page.locator('.graph-group-tabs [role="tab"]').nth(1).getAttribute('data-graph-group');
    const value = /** @returns {Promise<string>} */ () => page.locator('.graph-zoom-value').innerText();
    const before = await value();
    await page.locator('[data-zoom="in"]').click();
    await page.waitForFunction(/** @param {string} before @returns {boolean} */ before => document.querySelector('.graph-zoom-value')?.textContent !== before, before);
    const changed = await value();
    const pan = page.locator('.group-graph-scroll');
    const bounds = await pan.boundingBox();
    assert.ok(bounds);
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.wheel(160, 120);
    await page.waitForTimeout(150);
    const scroll = await pan.evaluate(/** @param {Element} element @returns {number[]} */ element => [element.scrollTop, element.scrollLeft]);
    await this.polls(page, 3);
    assert.equal(await value(), changed);
    const after = await pan.evaluate(/** @param {Element} element @returns {number[]} */ element => [element.scrollTop, element.scrollLeft]);
    assert.deepEqual(after, scroll, 'Active group pan survives live polls.');
    await page.locator('[data-graph-group]').nth(1).click();
    await page.waitForFunction(/** @param {string|null} id @returns {boolean} */ id => (/** @type {HTMLElement|null} */ (document.querySelector('.graph-group')))?.dataset.group === id, second);
    const secondBefore = await value();
    await page.locator('[data-zoom="in"]').click();
    await page.waitForFunction(/** @param {string} before @returns {boolean} */ before => document.querySelector('.graph-zoom-value')?.textContent !== before, secondBefore);
    const secondChanged = await value();
    await page.locator('[data-graph-group]').nth(0).click();
    await page.waitForFunction(/** @param {string|null} id @returns {boolean} */ id => (/** @type {HTMLElement|null} */ (document.querySelector('.graph-group')))?.dataset.group === id, first);
    assert.equal(await value(), changed, 'Other group zoom never changes this one.');
    assert.deepEqual(await page.locator('.group-graph-scroll').evaluate(/** @param {Element} element @returns {number[]} */ element => [element.scrollTop, element.scrollLeft]), scroll, 'Group pan restores after switching tabs.');
    await page.locator('[data-graph-group]').nth(0).focus();
    await page.keyboard.press('ArrowRight');
    await page.waitForFunction(/** @param {string|null} id @returns {boolean} */ id => (/** @type {HTMLElement|null} */ (document.querySelector('.graph-group')))?.dataset.group === id, second);
    assert.equal(await value(), secondChanged);
    await page.keyboard.press('Home');
    await page.waitForFunction(/** @param {string|null} id @returns {boolean} */ id => (/** @type {HTMLElement|null} */ (document.querySelector('.graph-group')))?.dataset.group === id, first);
    await page.locator('[data-zoom="fit"]').click();
    await page.waitForTimeout(200);
    const fit = await page.locator('.group-graph-scroll').evaluate(/** @param {Element} element @returns {{zoom:number,width:number,height:number,availableWidth:number,availableHeight:number}} */ element => {
      const canvas = element.querySelector('.graph-canvas');
      if (!canvas) throw new Error('The active graph canvas is missing.');
      const width = Number(canvas.getAttribute('width'));
      const zoom = width / Number(canvas.getAttribute('viewBox')?.split(' ')[2]);
      return {zoom, width, height:Number(canvas.getAttribute('height')),availableWidth:element.clientWidth,availableHeight:element.clientHeight};
    });
    assert.ok(fit.width <= fit.availableWidth && fit.height <= fit.availableHeight, 'Explicit Fit includes the complete tree.');
    const fitted = await value();
    await page.setViewportSize({width:1000,height:700});
    await page.waitForTimeout(200);
    assert.equal(await value(), fitted, 'Resize preserves manually chosen Fit/zoom.');
    await page.setViewportSize({width:1240,height:850});
    await page.waitForTimeout(200);
    this.report.checks.push({groupPan:{before:scroll,after},singleTreeTabs:{first,second,changed,secondChanged,fit,keyboard:true,manualResizePreserved:true}});
  }

  /** @param {import("playwright").Page} page @returns {Promise<void>} */
  static async graphShell(page) {
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelectorAll('.graph-group').length === 1, undefined, {timeout: 3000});
    await page.evaluate(/** @returns {Promise<number>} */ () => new Promise(/** @param {(time: number | PromiseLike<number>) => void} resolve @returns {number} */ resolve => requestAnimationFrame(/** @param {number} _time @returns {number} */ _time => requestAnimationFrame(resolve))));
    assert.equal(await page.locator('.graph-group').count(),1,'Only the active graph group is rendered.');
    assert.equal(await page.locator('.graph-group-tab').count(),4,'All groups have accessible tabs.');
    assert.equal(await page.locator('.graph-group-tab[aria-selected="true"]').count(),1);
    const layout = await page.locator('.graph-grid').evaluate(/** @param {Element} grid @returns {{width:number,height:number,groupWidth:number,groupHeight:number}} */ grid => {
      const group=grid.querySelector('.graph-group');if(!group)throw new Error('The active graph group is missing.');const a=grid.getBoundingClientRect(),b=group.getBoundingClientRect();
      return {width:a.width,height:a.height,groupWidth:b.width,groupHeight:b.height};
    });
    this.report.checks.push({singleFullAreaTree:layout});
    assert.ok(Math.abs(layout.width-layout.groupWidth)<2 && Math.abs(layout.height-layout.groupHeight)<6,'Active tree fills available graph area: '+JSON.stringify(layout));
    const tabs = await page.locator('.graph-group-tab').evaluateAll(/** @param {Element[]} elements @returns {OfflineTab[]} */ elements => elements.map(/** @param {Element} tab @returns {OfflineTab} */ tab => {
      const box = tab.getBoundingClientRect();
      const range = document.createRange(); range.selectNodeContents(tab);
      const text = [...range.getClientRects()].map(/** @param {DOMRect} rect @returns {OfflineTextBounds} */ rect => ({top:rect.top,bottom:rect.bottom,left:rect.left,right:rect.right}));
      const styles = getComputedStyle(tab);
      return {label:tab.textContent,height:box.height,top:box.top,bottom:box.bottom,left:box.left,right:box.right,
        lineHeight:styles.lineHeight,appearance:styles.appearance,text};
    }));
    this.report.checks.push({graphTabGeometry:tabs});
    assert.ok(tabs.every(/** @param {OfflineTab} tab @returns {boolean} */ tab => tab.height >= 38 && tab.lineHeight === '18px' && tab.appearance === 'none'), 'Tabs have explicit usable sizing and custom appearance.');
    assert.ok(tabs.every(/** @param {OfflineTab} tab @returns {boolean} */ tab => tab.text.every(/** @param {OfflineTextBounds} rect @returns {boolean} */ rect => rect.top >= tab.top + 1 && rect.bottom <= tab.bottom - 1 && rect.left >= tab.left + 1 && rect.right <= tab.right - 1)), 'Each group-tab label stays inside its own button border.');
  }

  /** @param {import("playwright").Page} page @param {string} name @param {string} unit @param {boolean} gaps @returns {Promise<void>} */
  static async charts(page, name, unit, gaps) {
    await page.locator('.chart-tabs').getByText(name, {exact: true}).click();
    await page.waitForFunction(/** @param {string} unit @returns {boolean} */ unit => {
      const canvas = document.querySelector('.resource-chart');
      return (/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(canvas)?.options.scales.y.title.text === unit;
    }, unit);
    const charts = await page.locator('.resource-chart').evaluateAll(/** @param {Element[]} canvases @returns {OfflineChartGeometry[]} */ canvases => canvases.map(/** @param {Element} canvas @returns {OfflineChartGeometry} */ canvas => {
      const chart = (/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(canvas);
      if(!chart)throw new Error('The resource chart is missing.');
      return {width: canvas.clientWidth, height: canvas.clientHeight, unit: chart.options.scales.y.title.text,
        window: chart.options.scales.x.max - chart.options.scales.x.min,
        title: chart.options.scales.x.title.text, gaps: chart.data.datasets.some(/** @param {import('../../app/client/models/ChartDataset.ts').ChartDataset} series @returns {boolean} */ series => series.data.some(/** @param {import('../../app/client/models/ChartPoint.ts').ChartPoint} point @returns {boolean} */ point => point.y === null)),
        points: chart.data.datasets[0].data.length, spanGaps: chart.data.datasets[0].spanGaps};
    }));
    assert.ok(charts.every(/** @param {OfflineChartGeometry} chart @returns {boolean} */ chart => chart.unit === unit && chart.window === 120 && chart.points === 61 && chart.height >= 180));
    if (gaps) assert.ok(charts.every(/** @param {OfflineChartGeometry} chart @returns {boolean} */ chart => chart.gaps && chart.spanGaps === false));
    const canvas = page.locator('.resource-chart').first();
    const rect = await canvas.boundingBox();
    assert.ok(rect);
    await page.mouse.move(rect.x + rect.width * 0.8, rect.y + rect.height * 0.4);
    await page.waitForFunction(/** @returns {boolean} */ () => ((/** @type {OfflineBrowser} */ (globalThis)).Chart?.getChart(document.querySelector('.resource-chart'))?.tooltip.opacity ?? 0) > 0);
    this.report.checks.push({charts: name, values: charts, hover: true});
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
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('[data-native-failure]') !== null,undefined,{timeout:6000});
    assert.equal(await this.samples(page),before,'A failed collection retains last measured sample count.');
    await page.waitForFunction(/** @returns {boolean} */ () => document.querySelector('[data-native-failure]') === null,undefined,{timeout:6000});
    await this.polls(page,1);
    this.report.checks.push('Automatic launch/cadence collection across three polls; failure keeps last sample and next poll recovers without manual controls');
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
      const selector = '.metric-card .metric-label,.metric-card p,.metric-card strong,.logical-agent-row h3,.assignment strong,.current-action strong,.runtime-card h2,.runtime-card-note,.activity-card h3,.history-card h3,.session-title strong,.graph-name,.graph-measurement';
      return [...document.querySelectorAll(selector)].flatMap(/** @param {Element} element @returns {OfflineClipping[]} */ element => {
        const card = element.closest('w-card,.logical-agent-row,.table-branch,.graph-node');
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
