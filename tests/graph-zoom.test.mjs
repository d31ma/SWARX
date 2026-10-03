import assert from 'node:assert/strict';
import { GraphGroupRenderer } from '../app/client/shared/scripts/GraphGroupRenderer.js';
import { GraphRenderer } from '../app/client/shared/scripts/GraphRenderer.js';
import { ViewContinuity } from '../app/client/shared/scripts/ViewContinuity.js';
import { OverviewFixtureClient } from './OverviewFixtureClient.mjs';

/** @typedef {import('../app/client/models/RuntimeView.ts').RuntimeView} RuntimeView */
/** @typedef {import('../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {import('../app/client/models/ObservatoryModel.ts').ObservatoryModel} ObservatoryModel */
/** @typedef {{pid: number, parent: number, started: string, name: string, cpu_percent: number | null, resident_bytes: number | null, received_per_second?: number | null, sent_per_second?: number | null, disk_read_per_second?: number | null, disk_write_per_second?: number | null}} FixtureProcess */
/** @typedef {FixtureProcess & {depth: number, indentClass: string, memory: string, relationship: string}} FixtureRow */
/** @typedef {{id: string, pid: number, started: string, name: string, family: string, scope: string, shared: boolean, instrumented: boolean, metrics: null, tasks: [], terminals: [], processes: FixtureProcess[], tree: FixtureRow[]}} FixtureGroup */
/** @typedef {{model: Pick<ObservatoryModel, 'sessions' | 'runtimes' | 'selected'>, pid: number, graphGroup?: string, showGraph?: (event: Event, id: string) => void}} FixtureSelection */
/** @typedef {{view: TestElement, stage: TestElement, selection: FixtureSelection}} GraphFixture */
/** @typedef {{x: number, y: number}} Point */
/** @typedef {Point & {pid: number, width: number, height: number}} PositionedCard */
/** @typedef {{activeElement: TestElement | null, createElement: (tag: string) => TestElement, createElementNS: (namespace: string, tag: string) => TestElement}} TestDocument */

class TestElement {
  /** @param {string} [tag] */
  constructor(tag = 'div') {
    /** @type {string} */
    this.tagName = tag;
    /** @type {TestElement[]} */
    this.children = [];
    /** @type {TestElement | null} */
    this.parentElement = null;
    /** @type {{[name: string]: string | undefined}} */
    this.dataset = {};
    /** @type {{[name: string]: string}} */
    this.style = {};
    /** @type {Map<string, string>} */
    this.attributes = new Map();
    /** @type {Map<string, EventListener>} */
    this.listeners = new Map();
    /** @type {string} */
    this.className = '';
    /** @type {string} */
    this.textContent = '';
    /** @type {number} */
    this.scrollTop = 0;
    /** @type {number} */
    this.scrollLeft = 0;
    /** @type {number | null} */
    this._clientWidth = null;
    /** @type {number} */
    this.clientHeight = 400;
    /** @type {{add: (name: string) => void, toggle: (name: string, selected: boolean) => void}} */
    this.classList = {add: /** @param {string} name @returns {void} */ name => { this.className += ' ' + name; }, toggle: /** @param {string} name @param {boolean} selected @returns {void} */ (name, selected) => {
      /** @type {string} */
    this.className = this.className.split(' ').filter(/** @param {string} value @returns {boolean} */ value => value !== name).join(' ');
      if (selected) this.className += ' ' + name;
    }};
  }
  /** @returns {number} */
  get clientWidth() { return this._clientWidth ?? this.parentElement?.clientWidth ?? 300; }
  /** @param {number} value */
  set clientWidth(value) { this._clientWidth = value; }
  /** @param {TestElement[]} elements @returns {void} */
  append(...elements) { elements.forEach(/** @param {TestElement} element @returns {void} */ element => { element.parentElement = this; this.children.push(element); }); }
  /** @param {TestElement[]} elements @returns {void} */
  replaceChildren(...elements) { this.children = []; this.append(...elements); }
  /** @param {TestElement} element @returns {void} */
  replaceWith(element) { const parent=this.parentElement; assert.ok(parent); element.parentElement=parent; parent.children[parent.children.indexOf(this)]=element; }
  /** @param {string} name @param {string} value @returns {void} */
  setAttribute(name, value) { this.attributes.set(name, value); }
  /** @param {string} name @returns {string | null} */
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  /** @param {string} selector @returns {boolean} */
  matches(selector) {
    if (selector.includes(',')) return selector.split(',').some(/** @param {string} part @returns {boolean} */ part => this.matches(part.trim()));
    if (selector.startsWith('.')) return this.className.split(' ').includes(selector.slice(1));
    const attribute = selector.match(/^\[data-(\w+)(?:="([^"]+)")?\]$/);
    return attribute ? this.dataset[attribute[1]] !== undefined && (attribute[2] === undefined || this.dataset[attribute[1]] === attribute[2]) : false;
  }
  /** @param {string} selector @returns {TestElement[]} */
  querySelectorAll(selector) { return this.children.flatMap(/** @param {TestElement} child @returns {TestElement[]} */ child => [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)]); }
  /** @param {string} selector @returns {TestElement | null} */
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  /** @param {string} selector @returns {TestElement | null} */
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) ?? null; }
  /** @returns {void} */
  focus() { /** @type {TestDocument} */ (/** @type {unknown} */ (document)).activeElement = this; }
  /** @param {string} name @param {EventListener} callback @returns {void} */
  addEventListener(name, callback) { this.listeners.set(name,callback); }
  /** @param {string} name @returns {void} */
  removeEventListener(name) { this.listeners.delete(name); }
}

globalThis.document = /** @type {Document} */ (/** @type {unknown} */ ({
  activeElement: null,
  createElement: /** @param {string} tag @returns {TestElement} */ tag => new TestElement(tag),
  createElementNS: /** @param {string} _namespace @param {string} tag @returns {TestElement} */ (_namespace, tag) => new TestElement(tag),
}));
globalThis.HTMLElement = /** @type {typeof HTMLElement} */ (/** @type {unknown} */ (TestElement));
globalThis.ResizeObserver = /** @type {typeof ResizeObserver} */ (/** @type {unknown} */ (class { /** @returns {void} */ observe() {} /** @returns {void} */ disconnect() {} }));
globalThis.MutationObserver = /** @type {typeof MutationObserver} */ (/** @type {unknown} */ (class {
  /** @returns {void} */
  observe() {}
  /** @returns {void} */
  disconnect() {}
}));

/** @param {TestElement} element @returns {HTMLElement} */
function domView(element) { return /** @type {HTMLElement} */ (/** @type {unknown} */ (element)); }
/** @param {FixtureSelection} selection @returns {ViewSelection} */
function rendererSelection(selection) { return /** @type {ViewSelection} */ (/** @type {unknown} */ (selection)); }
/** @param {Element | null} element @returns {TestElement} */
function testElement(element) { assert.ok(element); return /** @type {TestElement} */ (/** @type {unknown} */ (element)); }
/** @param {Point[]} values @returns {Point} */
function finalPoint(values) { const last = values.at(-1); assert.ok(last); return last; }

/** @param {TestElement} view @param {string} selector @returns {TestElement} */
function selectedElement(view, selector) { const found = view.querySelector(selector);assert.ok(found);return found; }

/** @param {FixtureGroup} group @param {string} selected @returns {RuntimeView} */
function runtimeView(group, selected) {
  const nativeGroup = {...group, status:'process-present', task_status:'unavailable',
    identity_evidence:'Synthetic graph fixture only', members:group.processes.map(/** @param {FixtureProcess} process @returns {number} */ process => process.pid), history:[]};
  const model = OverviewFixtureClient.project({now:120, sampled_at:120, started_at:0,
    totals:{cpu_seconds:0, received_bytes:0, sent_bytes:0, disk_read_bytes:0, disk_written_bytes:0,
      peak_resident_bytes:0, resident_bytes:0, samples:0}, groups:[nativeGroup], logical_agents:[], activity:[],
    errors:[], limitations:['Synthetic graph fixture only'], selection:{selected, query:'', pid:null}});
  return model.sessions[0];
}

/** @param {string} id @param {number} pid @returns {RuntimeView} */
function group(id, pid) {
  return runtimeView({id, pid, started:'123:456', name:'Test Codex ' + pid, family:'Codex', scope:'shared-runtime',
    shared:true, instrumented:false, metrics:null, tasks:[], terminals:[],
    processes:[{pid,parent:1,started:'123:456',name:'codex',cpu_percent:1,resident_bytes:1024},
      {pid:pid+1,parent:pid,started:'123:457',name:'zsh',cpu_percent:0,resident_bytes:2048}],
    tree:[{pid,parent:1,started:'123:456',name:'codex',cpu_percent:1,resident_bytes:1024,depth:0,indentClass:'tree-depth-0',memory:'1.0 KiB',relationship:'PPID 1'},
      {pid:pid+1,parent:pid,started:'123:457',name:'zsh',cpu_percent:0,resident_bytes:2048,depth:1,indentClass:'tree-depth-1',memory:'2.0 KiB',relationship:'PPID '+pid}]}, 'codex:10:123:456');
}

/** @returns {GraphFixture} */
function fixture() {
  const view = new TestElement();
  const stage = new TestElement();
  stage.className = 'graph-stage';
  view.append(stage);
  const groups = [group('codex:10:123:456',10), group('codex:20:123:456',20)];
  return {view,stage,selection:{model:{sessions:groups,runtimes:groups,selected:groups[0].id},pid:10}};
}

/** @param {TestElement} view @param {string} id @returns {TestElement} */
function panel(view, id) { const found = view.querySelectorAll('.graph-group').find(/** @param {TestElement} element @returns {boolean} */ element => element.dataset.group === id); assert.ok(found); return found; }
/** @param {TestElement} view @param {string} id @returns {string} */
function scale(view, id) { return selectedElement(panel(view,id), '.graph-canvas').style.transform; }
/** @param {GraphFixture} input @returns {void} */
function draw(input) { GraphRenderer.draw(domView(input.view),rendererSelection(input.selection),true); }

const input = fixture();
const first = input.selection.model.sessions[0].id;
const second = input.selection.model.sessions[1].id;
/** @param {string} id @returns {void} */
function show(id) { input.selection.graphGroup=id; draw(input); }
/** @param {string} id @returns {number} */
function zoomNumber(id) { const matched = scale(input.view,id).match(/scale\((.*)\)/);assert.ok(matched);return Number(matched[1]); }
draw(input);
assert.equal(input.view.querySelectorAll('.graph-group').length,1);
assert.equal(input.view.querySelectorAll('.graph-group-tab').length,2);
const initialFirst=zoomNumber(first);
GraphRenderer.zoom(domView(input.view),first,'in'); draw(input);
const changedFirst=zoomNumber(first);
assert.equal(changedFirst,initialFirst+0.1);
show(second);
const initialSecond=zoomNumber(second);
GraphRenderer.zoom(domView(input.view),second,'out'); draw(input);
const changedSecond=zoomNumber(second);
assert.equal(changedSecond,initialSecond-0.1);
show(first); assert.equal(zoomNumber(first),changedFirst);
show(second); assert.equal(zoomNumber(second),changedSecond);
input.selection.model.sessions[1].processes[0].cpu_percent=2;
GraphRenderer.draw(domView(input.view),rendererSelection(input.selection));
assert.equal(zoomNumber(second),changedSecond);
GraphRenderer.zoom(domView(input.view),second,'fit'); draw(input);
const fittedCanvas=selectedElement(panel(input.view,second), '.graph-canvas');
const fitted=Math.min(1,296/parseFloat(fittedCanvas.style.width),396/parseFloat(fittedCanvas.style.height));
assert.ok(Math.abs(zoomNumber(second)-fitted)<0.001,'Adaptive fit finds the largest feasible scale within bounded search precision.');
const fittedZoom=zoomNumber(second);
assert.equal(selectedElement(panel(input.view,second), '[data-zoom="fit"]').getAttribute('aria-label'),'Fit process tree for Test Codex 20');
const scroll=selectedElement(panel(input.view,second), '.group-graph-scroll'); scroll.scrollTop=70; scroll.scrollLeft=12;
selectedElement(panel(input.view,second), '[data-zoom="in"]').focus(); draw(input);
assert.equal(selectedElement(panel(input.view,second), '.group-graph-scroll').scrollTop,70);
assert.equal(testElement(document.activeElement).dataset.zoom,'in');
GraphRenderer.zoom(domView(input.view),'missing:pid:start','in');
GraphRenderer.zoom(domView(input.view),second,'unknown'); draw(input); assert.equal(zoomNumber(second),fittedZoom);
input.selection.model.sessions[0]=group('codex:10:NEW-START',10);
show('codex:10:NEW-START'); assert.equal(zoomNumber('codex:10:NEW-START'),initialFirst);
show(second); assert.equal(zoomNumber(second),fittedZoom);
for(let i=0;i<20;i++) GraphRenderer.zoom(domView(input.view),second,'in');
draw(input); assert.equal(zoomNumber(second),1.5);
for(let i=0;i<20;i++) GraphRenderer.zoom(domView(input.view),second,'out');
draw(input); assert.equal(zoomNumber(second),0.001);
selectedElement(panel(input.view,second), '.group-graph-scroll').clientWidth=0;
GraphRenderer.zoom(domView(input.view),second,'fit');draw(input);assert.equal(zoomNumber(second),0.001);
const lifecycle=new AbortController();
input.selection.showGraph=/** @param {Event} _event @param {string} id @returns {void} */ (_event,id)=>{input.selection.graphGroup=id;draw(input);};
const continuity=new ViewContinuity(domView(input.view),lifecycle.signal,/** @returns {ViewSelection} */ ()=>rendererSelection(input.selection));continuity.start();
const localButton=selectedElement(panel(input.view,second), '[data-zoom="in"]'); localButton.focus();
const clicked=input.view.listeners.get('click');assert.ok(clicked);clicked(/** @type {Event} */ (/** @type {unknown} */ ({target:localButton})));assert.equal(zoomNumber(second),0.101);
continuity.dispose();
console.log('Graph zoom assertions passed: single active group, separate zoom state, live refresh, full-tree fit, focus/scroll, unknown action and PID reuse.');

/** @param {string} id @param {number} root @param {FixtureProcess[]} processes @returns {GraphFixture} */
function treeFixture(id, root, processes) {
  const tree = processes.map(/** @param {FixtureProcess} process @returns {FixtureRow} */ process => ({...process, depth:0, indentClass:'tree-depth-0',memory:'1.0 KiB',relationship:'PPID '+process.parent}));
  const item = runtimeView({id,pid:root,started:'tree:123',name:'Tree fixture',family:'Codex',scope:'shared-runtime',shared:true,instrumented:false,metrics:null,tasks:[],terminals:[],processes,tree},id);
  const view = new TestElement();
  const stage = new TestElement();
  stage.className = 'graph-stage';
  stage.clientWidth = 2400;
  view.append(stage);
  return {view,stage,selection:{model:{sessions:[item],runtimes:[item],selected:id},pid:root}};
}
/** @param {number} pid @param {number} parent @returns {FixtureProcess} */
function row(pid,parent) { return {pid,parent,name:'process-' + pid,started:'tree:123',cpu_percent:0,resident_bytes:1024}; }
/** @param {GraphFixture} input @param {number} pid @returns {TestElement} */
function node(input,pid) { const found = input.view.querySelectorAll('.graph-node').find(/** @param {TestElement} element @returns {boolean} */ element => element.dataset.pid === String(pid)); assert.ok(found); return found; }
/** @param {GraphFixture} input @param {number} pid @returns {Point & {width: number}} */
function point(input,pid) { const element=node(input,pid); return {x:parseFloat(element.style.left),y:parseFloat(element.style.top),width:parseFloat(element.style.width)}; }
const branching = treeFixture('codex:500:tree',500,[row(500,1),row(501,500),row(502,500),row(503,500),row(511,501),row(512,501)]);
draw(branching);
assert.equal(point(branching,501).y,point(branching,502).y);
assert.equal(point(branching,502).y,point(branching,503).y);
assert.ok(point(branching,500).y < point(branching,501).y);
assert.ok(point(branching,501).y < point(branching,511).y);
assert.equal(point(branching,511).y,point(branching,512).y);
assert.ok(point(branching,501).x + point(branching,501).width < point(branching,502).x);
assert.ok(point(branching,502).x + point(branching,502).width < point(branching,503).x);
assert.ok(point(branching,500).x>=point(branching,501).x && point(branching,500).x<=point(branching,503).x,'Parent is centered over its complete subtree footprint.');
assert.equal(point(branching,501).x,(point(branching,511).x+point(branching,512).x)/2);
assert.equal(branching.view.querySelectorAll('.shared-trunk').length,2);
assert.equal(branching.view.querySelectorAll('.child-branch').length,5);
assert.deepEqual(branching.view.querySelectorAll('.child-branch').map(/** @param {TestElement} edge @returns {number[]} */ edge=>[Number(edge.getAttribute('data-parent')),Number(edge.getAttribute('data-child'))]),[[500,501],[500,502],[500,503],[501,511],[501,512]]);
const originalPositions=branching.view.querySelectorAll('.graph-node').map(/** @param {TestElement} element @returns {(string | undefined)[]} */ element=>[element.dataset.pid,element.style.left,element.style.top]);
branching.selection.model.sessions[0].processes[0].cpu_percent=8;
draw(branching);
assert.deepEqual(branching.view.querySelectorAll('.graph-node').map(/** @param {TestElement} element @returns {(string | undefined)[]} */ element=>[element.dataset.pid,element.style.left,element.style.top]),originalPositions);

const disconnected=treeFixture('codex:600:tree',600,[row(601,602),row(602,1),row(603,999)]);
draw(disconnected);
assert.equal(disconnected.view.querySelectorAll('.graph-node').length,3);
assert.equal(point(disconnected,602).y,point(disconnected,601).y);
assert.equal(point(disconnected,602).y,point(disconnected,603).y);
assert.equal(disconnected.view.querySelectorAll('.child-branch').length,0);
assert.equal(disconnected.selection.model.sessions[0].graph.nodes[0].edgeStatus,'parent-after-node');

const forest=treeFixture('codex:610:tree',610,[row(612,1),row(611,612),row(613,999)]);
draw(forest);
assert.equal(forest.view.querySelectorAll('.graph-node').length,3);
assert.ok(point(forest,612).y < point(forest,611).y);
assert.equal(point(forest,612).y,point(forest,613).y);
assert.equal(forest.view.querySelectorAll('.child-branch').length,1);
assert.equal(selectedElement(forest.view, '.child-branch').getAttribute('data-parent'),'612');
assert.equal(selectedElement(forest.view, '.child-branch').getAttribute('data-child'),'611');

const cyclic=treeFixture('codex:700:tree',700,[row(700,702),row(701,700),row(702,701),row(703,703)]);
draw(cyclic);
assert.equal(cyclic.view.querySelectorAll('.graph-node').length,4);
assert.equal(cyclic.view.querySelectorAll('.child-branch').length,2);
assert.ok(point(cyclic,700).y < point(cyclic,701).y);
assert.ok(point(cyclic,701).y < point(cyclic,702).y);
assert.equal(point(cyclic,703).y,point(cyclic,700).y);
console.log('Downward tree assertions passed: sibling fanout, centered parents, real PPID edges, deterministic refresh, missing roots, orphans and cycles.');

/** @param {TestElement} element @returns {string} */
function textOf(element) {return [element.textContent,...element.children.map(textOf)].join(' ');}
const compact = treeFixture('codex:800:compact',800,[{...row(800,1),received_per_second:null,sent_per_second:null,disk_read_per_second:null,disk_write_per_second:null},{...row(801,800),received_per_second:0,sent_per_second:0,disk_read_per_second:0,disk_write_per_second:0},{...row(802,800),received_per_second:null,sent_per_second:2048,disk_read_per_second:1024,disk_write_per_second:0}]);
draw(compact);
const compactNodes=compact.view.querySelectorAll('.graph-node');
assert.ok(!textOf(compactNodes[0]).includes('↓'));assert.ok(!textOf(compactNodes[0]).includes('Disk'));
assert.ok(textOf(compactNodes[1]).includes('↓ 0 B/s'));assert.ok(!textOf(compactNodes[1]).includes('Disk'));
assert.ok(textOf(compactNodes[2]).includes('↑ 2.0 KiB/s'));assert.ok(!textOf(compactNodes[2]).includes('↓'));
assert.ok(textOf(compactNodes[2]).includes('Disk R 1.0 KiB/s'));assert.ok(!textOf(compactNodes[2]).includes('W 0'));
assert.equal(compact.selection.model.sessions[0].processes[1].disk_read_per_second,0);
console.log('Compact tree labels preserve missing/zero: network zero shown; unavailable network and idle/unavailable disk omitted.');

/** @param {FixtureProcess[]} processes @param {number} root @param {number} zoom @param {number} width @returns {TestElement} */
function adaptivePanel(processes, root, zoom, width) {
  const input=treeFixture('codex:'+root+':adaptive',root,processes);
  return testElement(new GraphGroupRenderer(input.selection.model.sessions[0],rendererSelection(input.selection),zoom,width).render());
}
/** @param {TestElement} panel @returns {PositionedCard[]} */
function positions(panel) {
  return panel.querySelectorAll('.graph-node').map(/** @param {TestElement} node @returns {PositionedCard} */ node=>({pid:Number(node.dataset.pid),x:parseFloat(node.style.left),y:parseFloat(node.style.top),width:parseFloat(node.style.width),height:parseFloat(node.style.minHeight)}));
}
/** @param {TestElement} panel @returns {number[][]} */
function edges(panel) {return panel.querySelectorAll('.child-branch').map(/** @param {TestElement} edge @returns {number[]} */ edge=>[Number(edge.getAttribute('data-parent')),Number(edge.getAttribute('data-child'))]);}
/** @param {PositionedCard[]} nodes @returns {void} */
function requireNoOverlap(nodes) {
  for(let i=0;i<nodes.length;i++) for(let j=i+1;j<nodes.length;j++) {
    const a=nodes[i],b=nodes[j];
    assert.ok(a.x+a.width<=b.x || b.x+b.width<=a.x || a.y+a.height<=b.y || b.y+b.height<=a.y,'Actual cards do not overlap: '+a.pid+'/'+b.pid);
  }
}
const broadProcesses=[row(900,1),...Array.from({length:12},/** @param {unknown} _ @param {number} i @returns {FixtureProcess} */ (_,i)=>row(901+i,900))];
const broadFull=adaptivePanel(broadProcesses,900,1,1100);
const broadOut=adaptivePanel(broadProcesses,900,.5,1100);
const fullPositions=positions(broadFull),outPositions=positions(broadOut);
assert.equal(new Set(fullPositions.slice(1).map(/** @param {PositionedCard} node @returns {number} */ node=>node.y)).size,4);
assert.equal(new Set(outPositions.slice(1).map(/** @param {PositionedCard} node @returns {number} */ node=>node.y)).size,2,'Zoom out reflows sibling rows rather than only scaling fixed positions.');
assert.notDeepEqual(fullPositions,outPositions);
assert.deepEqual(edges(broadFull),edges(broadOut));
assert.equal(broadFull.querySelectorAll('.shared-trunk').length,1);
assert.equal(broadOut.querySelectorAll('.shared-trunk').length,1);
requireNoOverlap(fullPositions);requireNoOverlap(outPositions);
assert.deepEqual(outPositions,positions(adaptivePanel(broadProcesses,900,.5,1100)),'Reflow stays deterministic across polls.');
const narrow=adaptivePanel(broadProcesses,900,1,500);
assert.equal(new Set(positions(narrow).slice(1).map(/** @param {PositionedCard} node @returns {number} */ node=>node.y)).size,12);
assert.ok(parseFloat(selectedElement(narrow, '.graph-canvas').style.width)<=500);
requireNoOverlap(positions(narrow));
const deepProcesses=Array.from({length:60},/** @param {unknown} _ @param {number} i @returns {FixtureProcess} */ (_,i)=>row(1000+i,i===0?1:999+i));
const deep=adaptivePanel(deepProcesses,1000,1,900);
const deepPositions=positions(deep);
assert.equal(edges(deep).length,59);
assert.equal(new Set(deepPositions.map(/** @param {PositionedCard} node @returns {number} */ node=>node.x)).size,1,'A single-child chain stays aligned without adding unused gutter width.');
requireNoOverlap(deepPositions);
const deepGroup=treeFixture('codex:1000:fit',1000,deepProcesses);
const deepRenderer=new GraphGroupRenderer(deepGroup.selection.model.sessions[0],rendererSelection(deepGroup.selection),1,900);
const deepFit=deepRenderer.fit(896,500);
const fittedDeep=new GraphGroupRenderer(deepGroup.selection.model.sessions[0],rendererSelection(deepGroup.selection),deepFit,896).measure();
assert.ok(fittedDeep.width*deepFit<=896 && fittedDeep.height*deepFit<=500);
assert.ok(deepFit>.001 && deepFit<.5);
console.log('Adaptive layout assertions passed: broad/narrow row reflow, unchanged PPID edges, one shared trunk, no overlapping cards, deterministic refresh and whole deep-tree Fit.');

/** @param {TestElement} path @returns {Point[]} */
function pathPoints(path) {
  const pathValue=path.getAttribute('d');assert.ok(pathValue);
  const matched=pathValue.match(/-?\d+(?:\.\d+)?/g);assert.ok(matched);
  const values=matched.map(Number);
  return Array.from({length:values.length/2},/** @param {unknown} _ @param {number} index @returns {Point} */ (_,index)=>({x:values[index*2],y:values[index*2+1]}));
}
/** @param {Point} left @param {Point} right @returns {boolean} */
function samePoint(left,right) {return left.x===right.x && left.y===right.y;}
/** @param {TestElement} panel @returns {void} */
function requireConnectedTrunks(panel) {
  const cards=positions(panel);
  const branches=panel.querySelectorAll('.child-branch');
  const trunks=panel.querySelectorAll('.shared-trunk');
  const buses=panel.querySelectorAll('.shared-bus');
  for(const trunk of trunks) {
    const parent=Number(trunk.getAttribute('data-parent'));
    const children=branches.filter(/** @param {TestElement} branch @returns {boolean} */ branch=>Number(branch.getAttribute('data-parent'))===parent);
    assert.ok(children.length>0,'No trunk is drawn from a leaf or empty child row.');
    const trunkPoints=pathPoints(trunk);
    const card=cards.find(/** @param {PositionedCard} card @returns {boolean} */ card=>card.pid===parent);assert.ok(card);
    assert.deepEqual(trunkPoints[0],{x:card.x+card.width/2,y:card.y+card.height},'Trunk starts exactly at the parent card.');
    const rows=new Set(children.map(/** @param {TestElement} branch @returns {number} */ branch=>pathPoints(branch)[0].y));
    if(rows.size===1) {
      assert.equal(trunkPoints.length,2,'One-row fanout has a direct trunk without an unused side-lane tail.');
      assert.equal(trunkPoints[1].x,trunkPoints[0].x);
      assert.equal(trunkPoints[1].y,pathPoints(children[0])[0].y);
    } else {
      const parentBuses=buses.filter(/** @param {TestElement} bus @returns {boolean} */ bus=>Number(bus.getAttribute('data-parent'))===parent);
      assert.ok(parentBuses.some(/** @param {TestElement} bus @returns {boolean} */ bus=>samePoint(pathPoints(bus)[0],finalPoint(trunkPoints))),'Wrapped trunk ends at a real child-row bus.');
      assert.equal(finalPoint(trunkPoints).y,Math.max(...children.map(/** @param {TestElement} branch @returns {number} */ branch=>pathPoints(branch)[0].y)),'Trunk does not continue beyond its final real child row.');
    }
    for(const branch of children) {
      const points=pathPoints(branch);
      const child=cards.find(/** @param {PositionedCard} card @returns {boolean} */ card=>card.pid===Number(branch.getAttribute('data-child')));assert.ok(child);
      assert.deepEqual(finalPoint(points),{x:child.x+child.width/2,y:child.y},'Each branch ends at its actual visible child card.');
      const connected=buses.filter(/** @param {TestElement} bus @returns {boolean} */ bus=>Number(bus.getAttribute('data-parent'))===parent).some(/** @param {TestElement} bus @returns {boolean} */ bus=>{
        const [left,right]=pathPoints(bus);
        return points[0].y===left.y && points[0].x>=left.x && points[0].x<=right.x;
      });
      const onFirstTrunk=trunkPoints.some(/** @param {Point} point @param {number} index @returns {boolean} */ (point,index)=>{
        const next=trunkPoints[index+1];
        return next && point.y===next.y && points[0].y===point.y && points[0].x>=Math.min(point.x,next.x) && points[0].x<=Math.max(point.x,next.x);
      });
      assert.ok(connected || onFirstTrunk,'Every branch starts on its parent shared bus or horizontal trunk.');
    }
  }
}
const singleLeaf=adaptivePanel([row(1100,1)],1100,1,900);
assert.equal(selectedElement(singleLeaf, '.graph-edges').children.length,0);
const singleRow=adaptivePanel([row(1200,1),row(1201,1200),row(1202,1200),row(1203,1200)],1200,1,1500);
requireConnectedTrunks(singleRow);
requireConnectedTrunks(branching.view);requireConnectedTrunks(disconnected.view);requireConnectedTrunks(cyclic.view);
requireConnectedTrunks(broadFull);requireConnectedTrunks(broadOut);requireConnectedTrunks(narrow);requireConnectedTrunks(deep);
const exitedChildren=adaptivePanel([row(900,1),row(901,900)],900,1,1100);
requireConnectedTrunks(exitedChildren);
assert.equal(exitedChildren.querySelectorAll('.shared-trunk').length,1);
assert.deepEqual(edges(exitedChildren),[[900,901]],'Exiting children remove their links without a leftover wrap lane.');
const exitedParent=adaptivePanel([row(901,900),row(902,900)],900,1,1100);
assert.equal(exitedParent.querySelectorAll('.shared-trunk').length,0);
assert.equal(exitedParent.querySelectorAll('.child-branch').length,0,'Missing parents do not produce dangling trunks between disconnected roots.');
console.log('Connector endpoint assertions passed: no single-row tail, no leaf/orphan/exit trunk, final wrapped bus termination, real child endpoints and reflow-safe shared links.');
