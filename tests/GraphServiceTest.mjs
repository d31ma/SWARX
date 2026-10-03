import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';

/** @typedef {import('../app/client/services/browser/GraphService.js').default} GraphService */
/** @typedef {import('../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {{dataset: {identity: string, depth: string, pid: string, started: string, session?: string}, style: Record<string, string>, offsetHeight: number, selected: boolean, classList: {owner?: GraphNode, toggle: (name: string, value: boolean) => void}, parentIdentity: string | null}} GraphNode */
/** @typedef {{dataset: {parentIdentity: string, childIdentity: string | undefined}, attributes: Record<string, string>, setAttribute: (name: string, value: string) => void}} GraphPath */
/** @typedef {{style: Record<string, string>}} GraphStyleElement */
/** @typedef {GraphStyleElement & {attributes: Record<string, string>, setAttribute: (name: string, value: string) => void}} GraphSvg */
/** @typedef {{textContent: string}} GraphLabel */
/** @typedef {{clientWidth: number, clientHeight: number}} GraphViewport */
/** @typedef {GraphStyleElement | GraphSvg | GraphLabel | GraphViewport} GraphElement */
/** @typedef {{dataset: {group: string, session: string}, nodes: GraphNode[], trunks: GraphPath[], branches: GraphPath[], svg: GraphSvg, canvas: GraphStyleElement, size: GraphStyleElement, label: GraphLabel, viewport: GraphViewport, querySelector: (selector: string) => GraphElement | null, querySelectorAll: (selector: string) => (GraphNode | GraphPath)[]}} GraphGroup */

class GraphServiceTest {
  /** @returns {GraphService} */
  static service() {
    const source = readFileSync(new URL('../app/client/services/browser/GraphService.js', import.meta.url), 'utf8')
      .replace(/^\s*@(?:Service|OnStop)\s*$/gm, '')
      .replace('export default class GraphService', 'class GraphService');
    const context = vm.createContext({});
    vm.runInContext(source + '\nglobalThis.Graph = GraphService;', context);
    return new context.Graph();
  }

  /** @param {string} identity @param {number} depth @param {string | null} parent @param {number} [height] @returns {GraphNode} */
  static node(identity, depth, parent, height = 160) {
    return {dataset: {identity, depth: String(depth), pid: identity.split(':')[0], started: identity.split(':')[1]},
      style: {}, offsetHeight: height, selected: false,
      classList: {
        /** @param {string} _name @param {boolean} value @returns {void} */
        toggle(_name, value) { /** @type {GraphNode} */ (this.owner).selected = value; }
      },
      parentIdentity: parent};
  }

  /** @param {string} parent @param {string | undefined} child @returns {GraphPath} */
  static path(parent, child) {
    return {dataset: {parentIdentity: parent, childIdentity: child}, attributes: {},
      /** @param {string} name @param {string} value @returns {void} */
      setAttribute(name, value) { this.attributes[name] = value; }};
  }

  /** @param {string} identity @param {number} [width] @returns {GraphGroup} */
  static group(identity, width = 650) {
    const nodes = [GraphServiceTest.node('10:birth', 0, null), GraphServiceTest.node('11:birth', 1, '10:birth'), GraphServiceTest.node('12:birth', 1, '10:birth')];
    for (const node of nodes) node.classList.owner = node;
    const trunks = [GraphServiceTest.path('10:birth', undefined)];
    const branches = [GraphServiceTest.path('10:birth', '11:birth'), GraphServiceTest.path('10:birth', '12:birth')];
    /** @type {GraphSvg} */
    const svg = {style: {}, attributes: {},
      /** @param {string} name @param {string} value @returns {void} */
      setAttribute(name, value) { this.attributes[name] = value; }};
    /** @type {GraphStyleElement} */
    const canvas = {style: {}};
    /** @type {GraphStyleElement} */
    const size = {style: {}};
    /** @type {GraphLabel} */
    const label = {textContent: ''};
    /** @type {GraphViewport} */
    const viewport = {clientWidth: width, clientHeight: 240};
    return {dataset: {group: identity, session: identity}, nodes, trunks, branches, svg, canvas, size, label, viewport,
      /** @param {string} selector @returns {GraphElement | null} */
      querySelector(selector) { return /** @type {Record<string, GraphElement>} */ ({'.group-graph-scroll': viewport, '.graph-canvas': canvas, '.group-graph-size': size, '.graph-edges': svg, '.graph-zoom-value': label})[selector] ?? null; },
      /** @param {string} selector @returns {(GraphNode | GraphPath)[]} */
      querySelectorAll(selector) { return /** @type {Record<string, (GraphNode | GraphPath)[]>} */ ({'.graph-node': nodes, '.shared-trunk': trunks, '.child-branch': branches})[selector] ?? []; }};
  }

  /** @param {GraphGroup[]} groups @returns {HTMLElement} */
  static view(groups) {
    return /** @type {HTMLElement} */ (/** @type {unknown} */ ({
      /** @param {string} selector @returns {GraphGroup[]} */
      querySelectorAll(selector) { assert.ok(['.graph-group', '.session-process-table .session-process-pick'].includes(selector)); return selector === '.graph-group' ? groups : []; }}));
  }

  /** @param {string} [session] @param {number} [pid] @param {string} [birth] @returns {ViewSelection} */
  static selection(session = 'Codex:real-id', pid = 11, birth = 'birth') {
    return {group: null, mode: 'cpu', session, sessionPid: pid, sessionStarted: birth};
  }
}

test('predeclared cards fan out below the root and share one trunk', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const service = GraphServiceTest.service();
  service.draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.equal(group.nodes[1].style.top, group.nodes[2].style.top);
  assert.notEqual(group.nodes[1].style.left, group.nodes[2].style.left);
  assert.ok(parseFloat(group.nodes[0].style.top) < parseFloat(group.nodes[1].style.top));
  assert.equal(group.trunks.length, 1);
  assert.match(group.trunks[0].attributes.d, /^M /);
  assert.match(group.branches[0].attributes.d, / Q /);
  assert.equal(group.nodes[1].selected, true);
  assert.equal(group.nodes[0].selected, false);
});

test('zoom belongs to one session and survives replacement across polls', /** @returns {void} */ () => {
  const first = GraphServiceTest.group('Codex:real-id');
  const second = GraphServiceTest.group('Claude:other-id');
  const view = GraphServiceTest.view([first, second]);
  const service = GraphServiceTest.service();
  service.draw(view, GraphServiceTest.selection());
  service.zoom(view, 'Codex:real-id', 'in');
  service.draw(view, GraphServiceTest.selection());
  assert.equal(first.canvas.style.transform, 'scale(1.1)');
  assert.equal(second.canvas.style.transform, 'scale(1)');
  const replacement = GraphServiceTest.group('Codex:real-id');
  service.draw(GraphServiceTest.view([replacement]), GraphServiceTest.selection());
  assert.equal(replacement.canvas.style.transform, 'scale(1.1)');
});

test('narrow cards wrap siblings and fit stays within the measured viewport', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id', 310);
  const view = GraphServiceTest.view([group]);
  const service = GraphServiceTest.service();
  service.draw(view, GraphServiceTest.selection());
  assert.notEqual(group.nodes[1].style.top, group.nodes[2].style.top);
  service.zoom(view, 'Codex:real-id', 'fit');
  service.draw(view, GraphServiceTest.selection());
  assert.ok(parseFloat(group.size.style.width) <= group.viewport.clientWidth);
  assert.ok(parseFloat(group.size.style.height) <= group.viewport.clientHeight);
});

test('PID reuse and session changes do not select another identity', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const service = GraphServiceTest.service();
  service.draw(GraphServiceTest.view([group]), GraphServiceTest.selection('Codex:real-id', 11, 'different-birth'));
  assert.equal(group.nodes.some(/** @param {GraphNode} node @returns {boolean} */ node => node.selected), false);
  service.draw(GraphServiceTest.view([group]), GraphServiceTest.selection('Claude:real-id'));
  assert.equal(group.nodes.some(/** @param {GraphNode} node @returns {boolean} */ node => node.selected), false);
});

test('missing endpoint leaves a gap rather than inventing a process edge', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  group.branches.push(GraphServiceTest.path('10:birth', '99:missing'));
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.equal(/** @type {GraphPath} */ (group.branches.at(-1)).attributes.d, '');
});

test('unmounted or hidden geometry is left alone', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id', 0);
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.deepEqual(group.canvas.style, {});
});

test('service disposal releases remembered zoom without touching another instance', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const view = GraphServiceTest.view([group]);
  const first = GraphServiceTest.service();
  const other = GraphServiceTest.service();
  first.zoom(view, 'Codex:real-id', 'in');
  other.zoom(view, 'Codex:real-id', 'out');
  first.dispose();
  first.draw(view, GraphServiceTest.selection());
  assert.equal(group.canvas.style.transform, 'scale(1)');
  other.draw(view, GraphServiceTest.selection());
  assert.equal(group.canvas.style.transform, 'scale(0.9)');
});

test('Table highlights only the selected session PID and birth', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const node = group.nodes[1];
  node.dataset.session = 'Codex:real-id';
  const view = /** @type {HTMLElement} */ (/** @type {unknown} */ ({
    /** @param {string} selector @returns {GraphNode[]} */
    querySelectorAll(selector) {return selector === '.session-process-table .session-process-pick' ? [node] : [];}
  }));
  const service = GraphServiceTest.service();
  service.draw(view, GraphServiceTest.selection());
  assert.equal(node.selected, true);
  service.draw(view, GraphServiceTest.selection('Codex:real-id', 11, 'reused-birth'));
  assert.equal(node.selected, false);
});
