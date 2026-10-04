import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';

/** @typedef {import('../../app/client/services/browser/GraphService.js').default} GraphService */
/** @typedef {import('../../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {{attributes: Record<string, string>, setAttribute: (name: string, value: string) => void}} GraphFrame */
/** @typedef {{dataset: {identity: string, depth: string, pid: string, started: string, session?: string}, style: Record<string, string>, parentElement: GraphFrame, offsetHeight: number, selected: boolean, classList: {owner?: GraphNode, toggle: (name: string, value: boolean) => void}, parentIdentity: string | null}} GraphNode */
/** @typedef {{dataset: {parentIdentity: string, childIdentity: string | undefined}, attributes: Record<string, string>, setAttribute: (name: string, value: string) => void}} GraphPath */
/** @typedef {{style: Record<string, string>}} GraphStyleElement */
/** @typedef {GraphStyleElement & {attributes: Record<string, string>, setAttribute: (name: string, value: string) => void}} GraphSvg */
/** @typedef {{textContent: string}} GraphLabel */
/** @typedef {{clientWidth: number, clientHeight: number}} GraphViewport */
/** @typedef {GraphStyleElement | GraphSvg | GraphLabel | GraphViewport} GraphElement */
/** @typedef {{dataset: {group: string, session: string}, nodes: GraphNode[], trunks: GraphPath[], branches: GraphPath[], svg: GraphSvg, label: GraphLabel, viewport: GraphViewport, querySelector: (selector: string) => GraphElement | null, querySelectorAll: (selector: string) => (GraphNode | GraphPath)[]}} GraphGroup */
/** @typedef {{x: number, y: number}} GraphPoint */
/** @typedef {{start: GraphPoint, end: GraphPoint}} GraphSegment */
/** @typedef {{identity: string, left: number, top: number, right: number, bottom: number}} GraphBox */
/** @typedef {{parent: string | undefined, child: string | undefined, crossed: string}} GraphCollision */
/** @typedef {{parent: string | undefined, child: string | undefined, reason: string}} GraphConnectionGap */

class GraphServiceTest {
  /** @returns {GraphService} */
  static service() {
    const source = readFileSync(new URL('../../app/client/services/browser/GraphService.js', import.meta.url), 'utf8')
      .replace(/^\s*@(?:Service|OnStop)\s*$/gm, '')
      .replace('export default class GraphService', 'class GraphService');
    const context = vm.createContext({});
    vm.runInContext(source + '\nglobalThis.Graph = GraphService;', context);
    return new context.Graph();
  }

  /** @param {string} identity @param {number} depth @param {string | null} parent @param {number} [height] @returns {GraphNode} */
  static node(identity, depth, parent, height = 160) {
    return {dataset: {identity, depth: String(depth), pid: identity.split(':')[0], started: identity.split(':')[1]},
      style: {}, parentElement: GraphServiceTest.frame(), offsetHeight: height, selected: false,
      classList: {
        /** @param {string} _name @param {boolean} value @returns {void} */
        toggle(_name, value) { /** @type {GraphNode} */ (this.owner).selected = value; }
      },
      parentIdentity: parent};
  }

  /** @returns {GraphFrame} */
  static frame() {
    return {attributes: {},
      /** @param {string} name @param {string} value @returns {void} */
      setAttribute(name, value) { this.attributes[name] = value; }};
  }

  /** @param {GraphGroup} group @returns {number} */
  static scale(group) {
    const base = Number(group.svg.attributes.viewBox.split(' ')[2]);
    return Math.round(Number(group.svg.attributes.width) / base * 1e6) / 1e6;
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
    /** @type {GraphLabel} */
    const label = {textContent: ''};
    /** @type {GraphViewport} */
    const viewport = {clientWidth: width, clientHeight: 240};
    return {dataset: {group: identity, session: identity}, nodes, trunks, branches, svg, label, viewport,
      /** @param {string} selector @returns {GraphElement | null} */
      querySelector(selector) { return /** @type {Record<string, GraphElement>} */ ({'.group-graph-scroll': viewport, '.graph-canvas': svg, '.graph-zoom-value': label})[selector] ?? null; },
      /** @param {string} selector @returns {(GraphNode | GraphPath)[]} */
      querySelectorAll(selector) { return /** @type {Record<string, (GraphNode | GraphPath)[]>} */ ({'.graph-node': nodes, '.shared-trunk': trunks, '.child-branch': branches})[selector] ?? []; }};
  }

  /** @param {GraphGroup[]} groups @returns {HTMLElement} */
  static view(groups) {
    return /** @type {HTMLElement} */ (/** @type {unknown} */ ({
      /** @param {string} selector @returns {GraphGroup[]} */
      querySelectorAll(selector) { assert.equal(selector, '.graph-group'); return groups; }}));
  }

  /** @param {string} [session] @param {number} [pid] @param {string} [birth] @returns {ViewSelection} */
  static selection(session = 'Codex:real-id', pid = 11, birth = 'birth') {
    return {group: null, mode: 'cpu', session, sessionPid: pid, sessionStarted: birth};
  }

  /** @returns {GraphGroup} */
  static denseGroup() {
    const group = this.group('Codex:real-id', 310);
    const nodes = [this.node('10:birth', 0, null, 160)];
    const branches = [];
    for (let identifier = 11; identifier <= 22; identifier += 1) {
      nodes.push(this.node(identifier + ':birth', 1, '10:birth', 160));
      branches.push(this.path('10:birth', identifier + ':birth'));
    }
    for (const node of nodes) node.classList.owner = node;
    group.nodes.splice(0, group.nodes.length, ...nodes);
    group.branches.splice(0, group.branches.length, ...branches);
    return group;
  }

  /** @returns {GraphGroup} */
  static separatedGroup() {
    const group = this.group('Codex:real-id', 310);
    const nodes = [this.node('10:birth', 0, null), this.node('11:birth', 1, '10:birth'),
      this.node('12:birth', 1, '10:birth'), this.node('13:birth', 2, '11:birth')];
    for (const node of nodes) node.classList.owner = node;
    group.nodes.splice(0, group.nodes.length, ...nodes);
    group.trunks.push(this.path('11:birth', undefined));
    group.branches.push(this.path('11:birth', '13:birth'));
    return group;
  }

  /** @param {string} identity @param {number} width @param {GraphNode[]} nodes @returns {GraphGroup} */
  static treeGroup(identity, width, nodes) {
    const group = this.group(identity, width);
    const branches = nodes.filter(/** @param {GraphNode} node @returns {boolean} */ node => node.parentIdentity !== null)
      .map(/** @param {GraphNode} node @returns {GraphPath} */ node => this.path(/** @type {string} */ (node.parentIdentity), node.dataset.identity));
    const parents = new Set(branches.map(/** @param {GraphPath} branch @returns {string} */ branch => branch.dataset.parentIdentity));
    const trunks = Array.from(parents, /** @param {string} parent @returns {GraphPath} */ parent => this.path(parent, undefined));
    for (const node of nodes) node.classList.owner = node;
    group.nodes.splice(0, group.nodes.length, ...nodes);
    group.trunks.splice(0, group.trunks.length, ...trunks);
    group.branches.splice(0, group.branches.length, ...branches);
    return group;
  }

  /** @param {GraphGroup} group @returns {void} */
  static assertSeparatedCards(group) {
    const boxes = this.boxes(group);
    const [, , width, height] = group.svg.attributes.viewBox.split(' ').map(Number);
    for (const [index, box] of boxes.entries()) {
      assert.ok([box.left, box.top, box.right, box.bottom].every(Number.isFinite), 'A visible card has finite geometry: ' + box.identity);
      assert.ok(box.right > box.left && box.bottom - box.top >= group.nodes[index].offsetHeight, 'A card accommodates its measured content: ' + box.identity);
      assert.ok(box.left >= 0 && box.top >= 0 && box.right <= width && box.bottom <= height, 'The full card fits in its unscaled canvas: ' + box.identity);
      for (const other of boxes.slice(index + 1)) {
        assert.ok(box.right <= other.left || other.right <= box.left || box.bottom <= other.top || other.bottom <= box.top,
          'Visible cards do not overlap: ' + box.identity + '/' + other.identity);
      }
    }
  }

  /** @param {GraphGroup} group @returns {GraphBox[]} */
  static boxes(group) {
    return group.nodes.map(/** @param {GraphNode} node @returns {GraphBox} */ node => ({
      identity: node.dataset.identity,
      left: Number(node.parentElement.attributes.x), top: Number(node.parentElement.attributes.y),
      right: Number(node.parentElement.attributes.x) + Number(node.parentElement.attributes.width),
      bottom: Number(node.parentElement.attributes.y) + Number(node.parentElement.attributes.height)
    }));
  }

  /** @param {GraphGroup} group @returns {GraphCollision[]} */
  static routeCollisions(group) {
    /** @type {GraphCollision[]} */
    const collisions = [];
    const boxes = this.boxes(group);
    for (const path of [...group.trunks, ...group.branches]) {
      const segments = this.segments(path.attributes.d ?? '');
      for (const box of boxes) {
        if ([path.dataset.parentIdentity, path.dataset.childIdentity].includes(box.identity)) continue;
        if (segments.some(/** @param {GraphSegment} segment @returns {boolean} */ segment => this.crosses(segment, box))) {
          collisions.push({parent: path.dataset.parentIdentity, child: path.dataset.childIdentity, crossed: box.identity});
        }
      }
    }
    return collisions;
  }

  /** @param {GraphGroup} group @returns {GraphConnectionGap[]} */
  static connectionGaps(group) {
    /** @type {GraphConnectionGap[]} */
    const gaps = [];
    const boxes = this.boxes(group);
    for (const path of group.branches) {
      const parent = boxes.find(/** @param {GraphBox} box @returns {boolean} */ box => box.identity === path.dataset.parentIdentity);
      const child = boxes.find(/** @param {GraphBox} box @returns {boolean} */ box => box.identity === path.dataset.childIdentity);
      if (!parent || !child) continue;
      const segments = this.segments(path.attributes.d ?? '');
      if (segments.length === 0) {
        gaps.push({parent: path.dataset.parentIdentity, child: path.dataset.childIdentity, reason: 'Missing visible edge'});
        continue;
      }
      const first = segments[0].start;
      const last = segments[segments.length - 1].end;
      const trunk = group.trunks.find(/** @param {GraphPath} value @returns {boolean} */ value => value.dataset.parentIdentity === parent.identity);
      const trunkSegments = this.segments(trunk?.attributes.d ?? '');
      const startsOnTrunk = trunkSegments.some(/** @param {GraphSegment} segment @returns {boolean} */ segment => this.onSegment(first, segment));
      const parentOrigin = {x: (parent.left + parent.right) / 2, y: parent.bottom};
      const trunkStartsOnParent = this.samePoint(trunkSegments[0]?.start, parentOrigin);
      const endsOnChild = this.samePoint(last, {x: (child.left + child.right) / 2, y: child.top});
      if (![startsOnTrunk, trunkStartsOnParent, endsOnChild].every(Boolean)) {
        gaps.push({parent: path.dataset.parentIdentity, child: path.dataset.childIdentity, reason: 'Edge does not connect its parent trunk and child'});
      }
    }
    return gaps;
  }

  /** @param {string} path @returns {GraphSegment[]} */
  static segments(path) {
    /** @type {GraphSegment[]} */
    const segments = [];
    const tokens = path.match(/[A-Za-z]|-?\d*\.?\d+(?:e[-+]?\d+)?/gi) ?? [];
    let point = {x: 0, y: 0};
    let index = 0;
    while (index < tokens.length) {
      const command = tokens[index++];
      const count = /** @type {Record<string, number>} */ ({M: 2, L: 2, H: 1, V: 1, Q: 4, C: 6})[command];
      assert.notEqual(count, undefined, 'Unsupported SVG geometry command: ' + command);
      const values = tokens.slice(index, index + count).map(Number);
      index += count;
      const points = this.commandPoints(command, point, values);
      if (command !== 'M') {
        for (const end of points) {
          segments.push({start: point, end});
          point = end;
        }
      }
      point = points[points.length - 1];
    }
    return segments;
  }

  /** @param {string} command @param {GraphPoint} point @param {number[]} values @returns {GraphPoint[]} */
  static commandPoints(command, point, values) {
    switch (command) {
      case 'H': return [{x: values[0], y: point.y}];
      case 'V': return [{x: point.x, y: values[0]}];
      case 'M':
      case 'L': return [{x: values[0], y: values[1]}];
      default: return this.curvePoints(point, values);
    }
  }

  /** @param {GraphPoint} start @param {number[]} values @returns {GraphPoint[]} */
  static curvePoints(start, values) {
    /** @type {GraphPoint[]} */
    const points = [];
    for (let index = 1; index <= 32; index += 1) {
      const progress = index / 32;
      const remaining = 1 - progress;
      if (values.length === 4) {
        points.push({x: remaining ** 2 * start.x + 2 * remaining * progress * values[0] + progress ** 2 * values[2],
          y: remaining ** 2 * start.y + 2 * remaining * progress * values[1] + progress ** 2 * values[3]});
        continue;
      }
      points.push({x: remaining ** 3 * start.x + 3 * remaining ** 2 * progress * values[0] + 3 * remaining * progress ** 2 * values[2] + progress ** 3 * values[4],
        y: remaining ** 3 * start.y + 3 * remaining ** 2 * progress * values[1] + 3 * remaining * progress ** 2 * values[3] + progress ** 3 * values[5]});
    }
    return points;
  }

  /** @param {GraphSegment} segment @param {GraphBox} box @returns {boolean} */
  static crosses(segment, box) {
    const horizontal = segment.end.x - segment.start.x;
    const vertical = segment.end.y - segment.start.y;
    const boundaries = [[-horizontal, segment.start.x - box.left - 0.01],
      [horizontal, box.right - segment.start.x - 0.01],
      [-vertical, segment.start.y - box.top - 0.01], [vertical, box.bottom - segment.start.y - 0.01]];
    let first = 0;
    let last = 1;
    for (const [direction, distance] of boundaries) {
      if (direction === 0 && distance < 0) return false;
      if (direction === 0) continue;
      const progress = distance / direction;
      if (direction < 0) first = Math.max(first, progress);
      if (direction > 0) last = Math.min(last, progress);
      if (first > last) return false;
    }
    return true;
  }

  /** @param {GraphPoint} point @param {GraphSegment} segment @returns {boolean} */
  static onSegment(point, segment) {
    const horizontal = segment.end.x - segment.start.x;
    const vertical = segment.end.y - segment.start.y;
    const length = horizontal ** 2 + vertical ** 2;
    if (length === 0) return this.samePoint(point, segment.start);
    const progress = ((point.x - segment.start.x) * horizontal + (point.y - segment.start.y) * vertical) / length;
    if (progress < 0 || progress > 1) return false;
    return this.samePoint(point, {x: segment.start.x + progress * horizontal, y: segment.start.y + progress * vertical});
  }

  /** @param {GraphPoint | undefined} first @param {GraphPoint} second @returns {boolean} */
  static samePoint(first, second) {
    return first !== undefined && Math.abs(first.x - second.x) < 0.01 && Math.abs(first.y - second.y) < 0.01;
  }
}

test('predeclared cards fan out below the root and share one trunk', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const service = GraphServiceTest.service();
  service.draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  const boxes = GraphServiceTest.boxes(group);
  assert.equal(boxes[1].top, boxes[2].top);
  assert.notEqual(boxes[1].left, boxes[2].left);
  assert.ok(boxes[0].top < boxes[1].top);
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
  assert.equal(GraphServiceTest.scale(first), 1.1);
  assert.equal(GraphServiceTest.scale(second), 1);
  const replacement = GraphServiceTest.group('Codex:real-id');
  service.draw(GraphServiceTest.view([replacement]), GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(replacement), 1.1);
});

test('narrow cards wrap siblings and fit stays within the measured viewport', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id', 310);
  const view = GraphServiceTest.view([group]);
  const service = GraphServiceTest.service();
  service.draw(view, GraphServiceTest.selection());
  assert.notEqual(GraphServiceTest.boxes(group)[1].top, GraphServiceTest.boxes(group)[2].top);
  service.zoom(view, 'Codex:real-id', 'fit');
  service.draw(view, GraphServiceTest.selection());
  assert.ok(Number(group.svg.attributes.width) <= group.viewport.clientWidth);
  assert.ok(Number(group.svg.attributes.height) <= group.viewport.clientHeight);
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
  assert.deepEqual(group.svg.attributes, {});
  assert.ok(group.nodes.every(/** @param {GraphNode} node @returns {boolean} */ node => Object.keys(node.parentElement.attributes).length === 0));
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
  assert.equal(GraphServiceTest.scale(group), 1);
  other.draw(view, GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(group), 0.9);
});

test('twelve narrow sibling branches connect true endpoints without crossing unrelated cards', /** @returns {void} */ () => {
  const group = GraphServiceTest.denseGroup();
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
});

test('a parent separated from its child by a wrapped peer routes around that peer', /** @returns {void} */ () => {
  const group = GraphServiceTest.separatedGroup();
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
});

test('normal fanout curves remain connected and avoid unrelated visible cards', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id', 650);
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
});

test('a missing child leaves no route while remaining real endpoints stay connected', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id', 650);
  group.nodes.splice(2, 1);
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.equal(group.branches[1].attributes.d, '');
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
});

test('a sixty-level chain keeps real endpoints aligned and Fit includes its deepest card', /** @returns {void} */ () => {
  const nodes = Array.from({length: 60}, /** @param {unknown} _value @param {number} depth @returns {GraphNode} */ (_value, depth) =>
    GraphServiceTest.node((1000 + depth) + ':birth', depth, depth === 0 ? null : (999 + depth) + ':birth'));
  const group = GraphServiceTest.treeGroup('Codex:deep-tree', 900, nodes);
  group.viewport.clientHeight = 500;
  const view = GraphServiceTest.view([group]);
  const service = GraphServiceTest.service();
  const selection = GraphServiceTest.selection('Codex:deep-tree', 1059);
  service.draw(view, selection);
  const boxes = GraphServiceTest.boxes(group);
  assert.equal(new Set(boxes.map(/** @param {GraphBox} box @returns {number} */ box => box.left)).size, 1);
  assert.equal(group.branches.filter(/** @param {GraphPath} path @returns {boolean} */ path => path.attributes.d.length > 0).length, 59);
  for (const [depth, box] of boxes.entries()) {
    if (depth > 0) assert.ok(boxes[depth - 1].bottom < box.top, 'Every descendant appears below its actual parent.');
  }
  assert.equal(group.nodes[59].selected, true);
  GraphServiceTest.assertSeparatedCards(group);
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
  service.zoom(view, 'Codex:deep-tree', 'fit');
  service.draw(view, selection);
  const scale = GraphServiceTest.scale(group);
  assert.ok(scale > 0.001 && scale < 0.5);
  assert.ok(Number(group.svg.attributes.width) <= group.viewport.clientWidth);
  assert.ok(Number(group.svg.attributes.height) <= group.viewport.clientHeight);
  assert.ok(GraphServiceTest.boxes(group)[59].bottom * scale <= group.viewport.clientHeight);
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
});

test('variable-height cards under multiple parents stay separate and connected across wide and narrow redraws', /** @returns {void} */ () => {
  const group = GraphServiceTest.treeGroup('Codex:variable-tree', 1500, [
    GraphServiceTest.node('10:birth', 0, null, 190),
    GraphServiceTest.node('11:birth', 1, '10:birth', 310),
    GraphServiceTest.node('12:birth', 1, '10:birth', 170),
    GraphServiceTest.node('13:birth', 2, '11:birth', 230),
    GraphServiceTest.node('14:birth', 2, '11:birth', 160),
    GraphServiceTest.node('15:birth', 2, '12:birth', 280),
    GraphServiceTest.node('16:birth', 2, '12:birth', 200)
  ]);
  const view = GraphServiceTest.view([group]);
  const service = GraphServiceTest.service();
  const selection = GraphServiceTest.selection('Codex:variable-tree', 13);
  service.draw(view, selection);
  const wide = GraphServiceTest.boxes(group);
  assert.equal(wide[1].top, wide[2].top);
  assert.equal(new Set(wide.slice(3).map(/** @param {GraphBox} box @returns {number} */ box => box.top)).size, 1);
  assert.equal(group.nodes[3].selected, true);
  GraphServiceTest.assertSeparatedCards(group);
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
  const paths = [...group.trunks, ...group.branches].map(/** @param {GraphPath} path @returns {string} */ path => path.attributes.d);
  service.draw(view, selection);
  assert.deepEqual(GraphServiceTest.boxes(group), wide);
  assert.deepEqual([...group.trunks, ...group.branches].map(/** @param {GraphPath} path @returns {string} */ path => path.attributes.d), paths);
  group.viewport.clientWidth = 390;
  service.draw(view, selection);
  const narrow = GraphServiceTest.boxes(group);
  assert.notDeepEqual(narrow, wide);
  assert.notEqual(narrow[1].top, narrow[2].top);
  assert.equal(new Set(narrow.slice(3).map(/** @param {GraphBox} box @returns {number} */ box => box.top)).size, 4);
  GraphServiceTest.assertSeparatedCards(group);
  assert.deepEqual(GraphServiceTest.connectionGaps(group), []);
  assert.deepEqual(GraphServiceTest.routeCollisions(group), []);
});

test('exited children and an absent parent clear their previous connector geometry', /** @returns {void} */ () => {
  const leaf = GraphServiceTest.group('Codex:leaf');
  const orphaned = GraphServiceTest.group('Codex:orphaned');
  const view = GraphServiceTest.view([leaf, orphaned]);
  const service = GraphServiceTest.service();
  service.draw(view, GraphServiceTest.selection());
  assert.ok([...leaf.trunks, ...leaf.branches, ...orphaned.trunks, ...orphaned.branches]
    .every(/** @param {GraphPath} path @returns {boolean} */ path => path.attributes.d.length > 0));
  leaf.nodes.splice(1);
  orphaned.nodes.splice(0, 1);
  service.draw(view, GraphServiceTest.selection());
  assert.ok([...leaf.trunks, ...leaf.branches, ...orphaned.trunks, ...orphaned.branches]
    .every(/** @param {GraphPath} path @returns {boolean} */ path => path.attributes.d === ''));
  GraphServiceTest.assertSeparatedCards(leaf);
  GraphServiceTest.assertSeparatedCards(orphaned);
});

test('unknown actions preserve zoom and repeated controls respect both bounds without changing a hidden group', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  const view = GraphServiceTest.view([group]);
  const service = GraphServiceTest.service();
  service.zoom(view, 'Codex:real-id', 'in');
  service.zoom(view, 'missing:identity', 'in');
  service.zoom(view, 'Codex:real-id', 'unknown');
  service.draw(view, GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(group), 1.1);
  for (let attempt = 0; attempt < 32; attempt += 1) service.zoom(view, 'Codex:real-id', 'in');
  service.draw(view, GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(group), 1.5);
  assert.equal(group.label.textContent, '150%');
  for (let attempt = 0; attempt < 32; attempt += 1) service.zoom(view, 'Codex:real-id', 'out');
  service.draw(view, GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(group), 0.001);
  group.viewport.clientWidth = 0;
  service.zoom(view, 'Codex:real-id', 'fit');
  service.draw(view, GraphServiceTest.selection());
  group.viewport.clientWidth = 650;
  service.draw(view, GraphServiceTest.selection());
  assert.equal(GraphServiceTest.scale(group), 0.001);
});

test('measured geometry writes SVG attributes and never an inline style', /** @returns {void} */ () => {
  const group = GraphServiceTest.group('Codex:real-id');
  GraphServiceTest.service().draw(GraphServiceTest.view([group]), GraphServiceTest.selection());
  assert.deepEqual(group.svg.style, {});
  assert.ok(group.nodes.every(/** @param {GraphNode} node @returns {boolean} */ node => Object.keys(node.style).length === 0));
  assert.deepEqual(Object.keys(group.nodes[0].parentElement.attributes).sort(), ['height', 'width', 'x', 'y']);
});
