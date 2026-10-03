import assert from 'node:assert/strict';
import { test } from 'node:test';
import { GraphGroupRenderer } from '../app/client/shared/scripts/GraphGroupRenderer.js';

/**
 * @typedef {{
 *   tag: string, children: GraphTestElement[], dataset: Record<string, string>,
 *   style: Record<string, string>, attributes: Map<string, string>, className: string,
 *   parentElement: GraphTestElement | null, id: string, tabIndex: number, textContent: string,
 *   append: (child: GraphTestElement) => void,
 *   setAttribute: (name: string, value: string) => void,
 *   classList: {add: (name: string) => void}
 * }} GraphTestElement
 */
/** @typedef {{pid: number, x: number, y: number, width: number, selected: boolean}} GraphTestPosition */

class GraphGroupRendererTest {
  /** @param {string} tag @returns {GraphTestElement} */
  static element(tag) {
    /** @type {GraphTestElement} */
    const element = {
      tag, children: [], dataset: {}, style: {}, attributes: new Map(), className: '',
      parentElement: null, id: '', tabIndex: 0, textContent: '',
      /** @param {GraphTestElement} child @returns {void} */
      append(child) { child.parentElement = this; this.children.push(child); },
      /** @param {string} name @param {string} value @returns {void} */
      setAttribute(name, value) { this.attributes.set(name, value); },
      classList: {
        /** @param {string} name @returns {void} */
        add(name) { element.className += ' ' + name; }
      }
    };
    return element;
  }

  /** @returns {void} */
  static browser() {
    const document = {
      /** @param {string} tag @returns {GraphTestElement} */
      createElement: tag => GraphGroupRendererTest.element(tag),
      /** @param {string | null} _namespace @param {string} tag @returns {GraphTestElement} */
      createElementNS: (_namespace, tag) => GraphGroupRendererTest.element(tag)
    };
    globalThis.document = /** @type {Document} */ (/** @type {unknown} */ (document));
  }

  /** @param {Element} element @param {string} name @returns {GraphTestElement[]} */
  static elements(element, name) {
    const fixture = /** @type {GraphTestElement} */ (/** @type {unknown} */ (element));
    return GraphGroupRendererTest.descendants(fixture, name);
  }

  /** @param {GraphTestElement} element @param {string} name @returns {GraphTestElement[]} */
  static descendants(element, name) {
    return element.children.flatMap(/** @param {GraphTestElement} child @returns {GraphTestElement[]} */ child => [
      ...child.className.split(' ').includes(name) ? [child] : [],
      ...GraphGroupRendererTest.descendants(child, name)
    ]);
  }

  /** @param {number} pid @param {number} parent @param {number} depth @param {string[]} children @returns {import('../app/client/models/ProcessGraphNode.ts').ProcessGraphNode} */
  static node(pid, parent, depth, children) {
    return { identity: pid + ':verified-test-birth', pid, parent, depth, graphDepth: depth,
      parentIdentity: null, childIdentities: children, name: 'Fixture process ' + pid,
      cpu: '0%', memory: '1 KiB', networkLabel: '', diskLabel: '',
      started: 'verified-test-birth', owner: null, indentClass: '', relationship: '',
      cpu_percent: null, resident_bytes: null, received_per_second: null, sent_per_second: null,
      disk_read_per_second: null, disk_write_per_second: null,
      edgeStatus: 'linked', edgeLabel: 'Explicit projected fixture relationship' };
  }

  /** @returns {import('../app/client/models/RuntimeView.ts').RuntimeView} */
  static group() {
    const nodes = [
      GraphGroupRendererTest.node(700, 1, 0, ['701:verified-test-birth', '702:verified-test-birth']),
      GraphGroupRendererTest.node(701, 700, 1, ['703:verified-test-birth']),
      GraphGroupRendererTest.node(702, 700, 1, []),
      GraphGroupRendererTest.node(703, 701, 2, [])
    ];
    return { id: 'fixture-group:700:birth', pid: 700, name: 'Synthetic projected graph fixture',
      family: 'Codex', scopeLabel: 'Shared runtime', cpu: '0%', memory: '4 KiB',
      graph: { roots: ['700:verified-test-birth'], nodes }, processes: nodes, tree: nodes,
      status: 'synthetic', scope: 'shared-runtime', shared: true, instrumented: false,
      metrics: null, history: [], charts: [], selected: '', providerHeader: false,
      download: '', upload: '', diskRead: '', diskWrite: '', networkNote: '', hookLabel: '',
      processCount: nodes.length, progress: '', tasks: [], terminals: [], cpuTotal: '',
      downloadTotal: '', uploadTotal: '', peakMemory: '', avatar: '', statusClass: '' };
  }

  /** @param {import('../app/client/models/RuntimeView.ts').RuntimeView} group @returns {import('../app/client/models/ObservatoryModel.ts').ObservatoryModel} */
  static model(group) {
    return {
      selected: group.id, detail: group, sessionGroups: [], linkedSessionCount: 0,
      harnessCount: 0, eventSourceCount: 0, snapshotSourceCount: 0, recoveredSourceCount: 0,
      sessions: [], runtimes: [], shared: [], logical: [], activity: [], logicalCount: 0,
      logicalFeed: '', logicalNotice: '', errors: [], limitations: [], cpuTotal: '',
      resident: '', peak: '', downloadTotal: '', uploadTotal: '', completed: 0, reported: 0,
      elapsed: '', samples: 0, sampled: '', sampleAge: 0, sessionCount: 0, sharedCount: 0,
      taskFeed: '', hasTaskFeed: false, ready: false, processLabel: '', liveStatus: ''
    };
  }

  /** @param {import('../app/client/models/RuntimeView.ts').RuntimeView} group @param {number} zoom @param {number} width @returns {GraphGroupRenderer} */
  static renderer(group, zoom = 1, width = 1200) {
    return new GraphGroupRenderer(group, {
      group: null, mode: 'Tree', model: GraphGroupRendererTest.model(group), pid: 703
    }, zoom, width);
  }

  /** @param {HTMLElement} panel @returns {GraphTestPosition[]} */
  static positions(panel) {
    return GraphGroupRendererTest.elements(panel, 'graph-node').map(/** @param {GraphTestElement} node @returns {GraphTestPosition} */ node => ({
      pid: Number(node.dataset.pid), x: parseFloat(node.style.left), y: parseFloat(node.style.top),
      width: parseFloat(node.style.width), selected: node.className.includes(' selected')
    }));
  }
}

test('projected identities and edges render readable deterministic downward cards', /** @returns {void} */ () => {
  GraphGroupRendererTest.browser();
  const group = GraphGroupRendererTest.group();
  const panel = GraphGroupRendererTest.renderer(group).render();
  const cards = GraphGroupRendererTest.positions(panel);
  assert.deepEqual(cards.map(/** @param {GraphTestPosition} card @returns {number} */ card => card.pid), [700, 701, 702, 703]);
  assert.equal(cards[1].y, cards[2].y);
  assert.ok(cards[0].y < cards[1].y && cards[1].y < cards[3].y);
  assert.ok(cards[1].x + cards[1].width < cards[2].x);
  assert.equal(cards[3].selected, true);
  assert.deepEqual(GraphGroupRendererTest.elements(panel, 'child-branch').map(/** @param {GraphTestElement} edge @returns {[string | undefined, string | undefined]} */ edge => [
    edge.attributes.get('data-parent-identity'), edge.attributes.get('data-child-identity')
  ]), [
    ['700:verified-test-birth', '701:verified-test-birth'],
    ['700:verified-test-birth', '702:verified-test-birth'],
    ['701:verified-test-birth', '703:verified-test-birth']
  ]);
  assert.equal(GraphGroupRendererTest.elements(panel, 'shared-trunk').length, 2);
  assert.deepEqual(cards, GraphGroupRendererTest.positions(GraphGroupRendererTest.renderer(group).render()));
});

test('narrow layouts wrap siblings and Fit measures the whole graph within the viewport', /** @returns {void} */ () => {
  GraphGroupRendererTest.browser();
  const group = GraphGroupRendererTest.group();
  const wide = GraphGroupRendererTest.positions(GraphGroupRendererTest.renderer(group).render());
  const narrow = GraphGroupRendererTest.positions(GraphGroupRendererTest.renderer(group, 1, 350).render());
  assert.equal(wide[1].y, wide[2].y);
  assert.notEqual(narrow[1].y, narrow[2].y);
  const zoom = GraphGroupRendererTest.renderer(group).fit(346, 250);
  const size = GraphGroupRendererTest.renderer(group, zoom, 346).measure();
  assert.ok(zoom > 0 && zoom < 1);
  assert.ok(size.width * zoom <= 346 && size.height * zoom <= 250);
});

test('a projected leaf has no fabricated connector and missing labels stay absent', /** @returns {void} */ () => {
  GraphGroupRendererTest.browser();
  const group = GraphGroupRendererTest.group();
  group.graph.nodes = [GraphGroupRendererTest.node(700, 1, 0, [])];
  const panel = GraphGroupRendererTest.renderer(group).render();
  assert.equal(GraphGroupRendererTest.elements(panel, 'graph-node').length, 1);
  assert.equal(GraphGroupRendererTest.elements(panel, 'shared-trunk').length, 0);
  assert.equal(GraphGroupRendererTest.elements(panel, 'child-branch').length, 0);
  assert.equal(GraphGroupRendererTest.elements(panel, 'graph-measurement').length, 1);
});
