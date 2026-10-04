import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

/** @typedef {{x: number, y: number, width: number, height: number}} QaRectangle */
/** @typedef {QaRectangle & {top: number, bottom: number}} QaBounds */
/** @typedef {{id: string, name: string, status: string, source: string, task: string, action?: string, sessionId?: string, taskId?: string, provider?: string, rawSource?: string}} QaLogicalRecord */
/** @typedef {{id: string, agent: string, sessionId: string, taskId: string, name: string, status: string, source: string, running: boolean}} QaActionRecord */
/** @typedef {{logicalText?: string, activityText?: string, overflowScroll?: boolean, actions?: QaActionRecord[], columns?: number, logicalRows?: QaLogicalRecord[], navigationDelay?: number, noGroups?: boolean, activityCards?: number, overflow?: boolean, width?: number, height?: number, nativeFailure?: boolean, advanceSamples?: boolean, advancePaused?: boolean, replaceOnLayout?: boolean}} QaOptions */
/** @typedef {{section: string, tab: string, drawer: boolean, opened: string, samples: number, live: string, clock: number, columns: number, activeGroup: number, groupClicks: number, groupPending: boolean, groupDropped: boolean}} QaState */
/** @typedef {{dataset: Record<string, string | undefined>}} QaBody */
/** @typedef {{label: string, action: string, check: string, section: string, mode: string, width: number, height: number, actionName?: string, agentRecordId?: string, sessionId?: string, taskId?: string, actionRecordId?: string}} QaInput */
/** @typedef {{verify: (input: QaInput, iconLoaded: boolean) => Promise<string>}} QaReporter */
/** @typedef {{selected: QaElement | null, selectNodeContents: (element: QaElement) => void, getBoundingClientRect: () => QaBounds}} QaRange */
/** @typedef {{body: QaBody, documentElement: {scrollWidth: number, scrollHeight: number}, createRange: () => QaRange, querySelectorAll: (selector: string) => QaElement[], querySelector: (selector: string) => QaElement | null}} QaDocument */
/** @typedef {{report: QaReporter, state: QaState, body: QaBody, graphCanvases: QaElement[], scrolls: QaElement[], graphs: QaElement[], context: import('node:vm').Context}} QaEnvironment */
/** @typedef {{id: string, scale: string}} QaScale */
/** @typedef {{id: string, top: number, left: number}} QaOffset */
/** @typedef {{baseline: {before: QaScale[], after: QaScale[], scroll: QaOffset | null}, current: QaScale[], scroll: QaOffset | null}} QaInteraction */
/** @typedef {{verification: {success: boolean}, logicalRecords: QaLogicalRecord[], interaction: QaInteraction | null, section: string, width: number, height: number, tableLayout: {columns: number[], cards: QaRectangle[]}, historyLayout: {columns: number[], cards: QaRectangle[]}, sampleCount: number, live: string, chartPoints: number, chartUnit: string}} QaResult */

/** Mirrors GraphService: width is the unscaled viewBox width times zoom. @param {QaElement} canvas @param {number} zoom @returns {void} */
function zoomCanvas(canvas,zoom){canvas.attributes.viewBox='0 0 100 100';canvas.attributes.width=String(100*zoom);}

class QaElement {
  /** @param {string} name @param {QaRectangle} [rect] */
  constructor(name, rect={x:0,y:0,width:400,height:200}) {
    /** @type {string} */
    this.textContent=name;
    /** @type {QaRectangle} */
    this.rect=rect;
    /** @type {Record<string, string>} */
    this.dataset={};
    /** @type {Record<string, string>} */
    this.style={};
    /** @type {string} */
    this.className='';
    /** @type {string} */
    this.innerHTML='';
    /** @type {number} */
    this.clientWidth=400;
    /** @type {number} */
    this.clientHeight=200;
    /** @type {number} */
    this.scrollWidth=400;
    /** @type {number} */
    this.scrollHeight=200;
    /** @type {number} */
    this.scrollTop=0;
    /** @type {number} */
    this.scrollLeft=0;
    /** @type {Record<string, string>} */
    this.attributes={};
    /** @type {(selector: string) => QaElement | null} */
    this.query=/** @param {string} _selector @returns {QaElement | null} */ _selector=>null;
    /** @type {() => void} */
    this.click=/** @returns {void} */ ()=>{};
  }
  /** @returns {QaBounds} */
  getBoundingClientRect(){return {...this.rect,top:this.rect.y,bottom:this.rect.y+this.rect.height};}
  /** @param {string} name @returns {string | null} */
  getAttribute(name){return this.attributes[name]??null;}
  /** @param {string} name @returns {boolean} */
  hasAttribute(name){return Object.hasOwn(this.attributes,name);}
  /** @returns {void} */
  scrollIntoView(){}
  /** @param {string} selector @returns {QaElement | null} */
  querySelector(selector){return this.query(selector);}
  /** @param {string} selector @returns {QaElement[]} */
  querySelectorAll(selector){const element=this.query(selector);return element===null?[]:[element];}
  /** @param {string} _selector @returns {QaElement | null} */
  closest(_selector){return null;}
  /** @returns {boolean} */
  dispatchEvent(){return true;}
}

/** @param {QaOptions} [options] @returns {QaEnvironment} */
function environment(options={}) {
  /** @type {QaState} */
  const state={section:"What's running",tab:'Resources',drawer:false,opened:'',samples:10,live:'Live',clock:0,columns:options.columns??2,activeGroup:0,groupClicks:0,groupPending:false,groupDropped:false};
  /** @type {QaBody} */
  const body={dataset:{platform:'macos',qaAgentId:'coord:00000000-0000-4000-8000-000000000001:/root'}};
  const table=new QaElement('table');
  const history=new QaElement('history');
  const sidebar=new QaElement('sidebar');
  const cards=Array.from({length:4},/** @param {unknown} _ @param {number} index @returns {QaElement} */ (_,index)=>new QaElement('card'+index,{x:20+index*420,y:100,width:400,height:180}));
  const canvas=new QaElement('chart');
  const records=options.logicalRows??[{id:'fixture-default-agent',name:'Fixture session',status:'running',source:'Fixture',task:'Fixture task'}];
  /** @type {QaElement[]} */
  const sessionControls=[];
  const logicalRows=records.map(/** @param {QaLogicalRecord} record @returns {QaElement} */ record=>{
    const row=new QaElement('agent');
    // Rows are read-only; the session card control opens the session panel.
    const control=new QaElement(record.name);
    control.click=/** @returns {void} */ ()=>queueMicrotask(/** @returns {void} */ ()=>{state.drawer=true;state.opened=record.id;state.section=state.tab;});
    sessionControls.push(control);
    row.closest=/** @param {string} selector @returns {QaElement | null} */ selector=>{
      if(selector!=='.session-card')return null;
      const card=new QaElement('session');
      card.query=/** @param {string} inner @returns {QaElement | null} */ inner=>{
        if(inner==='h3')return new QaElement(record.name);
        if(inner==='.session-card-open')return control;
        return null;
      };
      return card;
    };
    row.attributes['data-agent']=record.id;
    row.dataset.agent=record.id;
    row.dataset.session=record.sessionId??'00000000-0000-4000-8000-000000000001';
    row.dataset.task=record.taskId??'fixture-task';
    row.dataset.family=record.provider??'Codex';
    row.dataset.source=record.rawSource??record.source;
    row.dataset.status=record.status;
    row.query=/** @param {string} selector @returns {QaElement | null} */ selector=>{
      if(selector==='.agent-task')return new QaElement(record.task);
      if(selector==='.agent-tool-name')return new QaElement(record.action??'No tool running');
      if(selector==='.agent-tool.is-idle')return record.action?null:new QaElement('idle');
      return null;
    };
    return row;
  });
  const actionRows=(options.actions??[]).map(/** @param {QaActionRecord} record @returns {QaElement} */ record=>{
    const row=new QaElement(record.name+' '+record.status+' Measured by the reporting caller');
    row.dataset={action:record.id,agent:record.agent,session:record.sessionId,task:record.taskId,source:record.source,status:record.status,running:String(record.running)};
    row.query=/** @param {string} selector @returns {QaElement | null} */ selector=>{
      if(selector==='.action-history-heading strong')return new QaElement(record.name);
      if(selector==='.status-pill')return new QaElement(record.status);
      if(selector==='.action-elapsed'&&record.running)return new QaElement('Elapsed 1s');
      return null;
    };
    return row;
  });
  canvas.attributes['data-points']='12';
  const scrolls=[new QaElement('scroll'),new QaElement('scroll')];
  scrolls[0].scrollHeight=options.overflowScroll===false?200:600;
  scrolls[0].dataset.group='fixture-a';
  scrolls[1].dataset.group='fixture-b';
  const graphs=[new QaElement('graph'),new QaElement('graph')];
  const graphCanvases=[new QaElement('canvas'),new QaElement('canvas')];
  zoomCanvas(graphCanvases[0],1);
  zoomCanvas(graphCanvases[1],1);
  graphs[0].dataset.group='fixture-a';
  graphs[1].dataset.group='fixture-b';
  graphs.forEach(/** @param {QaElement} graph @param {number} index @returns {void} */ (graph,index)=>{
    graph.query=/** @param {string} selector @returns {QaElement} */ selector=>{
      if(selector==='.graph-canvas')return graphCanvases[index];
      if(selector==='.graph-name')return new QaElement('Fixture process');
      const button=new QaElement('zoom');
      button.click=/** @returns {void} */ ()=>{zoomCanvas(graphCanvases[index],1.1);};
      return button;
    };
  });
  const tabs=['Resources','Process tree','Activity'].map(/** @param {string} name @returns {QaElement} */ name=>{
    const element=new QaElement(name);
    element.click=/** @returns {void} */ ()=>queueMicrotask(/** @returns {void} */ ()=>{state.tab=name;state.section=name;});
    return element;
  });
  const close=new QaElement('×');
  close.click=/** @returns {void} */ ()=>queueMicrotask(/** @returns {void} */ ()=>{state.drawer=false;state.section="What's running";});
  const queryAll=/** @param {string} selector @returns {QaElement[]} */ selector=>{
    if(selector==='.agent-row')return logicalRows;
    if(selector==='.session-tab')return state.drawer?tabs:[];
    if(selector==='.session-card-open')return sessionControls;
    if(selector==='.action-history-row')return actionRows;
    if(selector==='[role=tab][data-session-tab]')return [];
    if(selector==='.session-process-card')return options.noGroups?[]:graphs;
    if(selector==='.session-process-pick')return cards;
    if(selector==='.graph-group')return options.noGroups||state.groupPending?[]:graphs;
    if(selector==='.group-graph-scroll')return scrolls;
    if(selector==='.session-process-grid .session-process-card')return cards;
    if(selector==='.context-resource-grid .session-process-card')return cards;
    if(selector==='.history-grid .history-card')return cards;
    if(selector==='.context-history-grid .history-card')return cards;
    if(selector==='.activity-grid .activity-card')return cards.slice(0, options.activityCards ?? 1);
    if(selector==='.history-card')return cards;
    if(selector==='.graph-node')return graphs;
    if(selector==='.session-row')return cards;
    return [];
  };
  /** @type {QaDocument} */
  const document={body,documentElement:{scrollWidth:options.overflow?1400:(options.width??1240),scrollHeight:options.height??850},
    createRange:/** @returns {QaRange} */ ()=>({
      /** @type {QaElement | null} */
      selected:null,
      /** @param {QaElement} element @returns {void} */
      selectNodeContents(element){this.selected=element;},
      /** @returns {QaBounds} */
      getBoundingClientRect(){assert.ok(this.selected);const rect=this.selected.getBoundingClientRect();return {...rect,top:rect.top+2,bottom:rect.bottom-2};}
    }),
    querySelectorAll:queryAll,
    querySelector:/** @param {string} selector @returns {QaElement | null} */ selector=>{
      if(selector==='.session-panel.is-open .session-tab[aria-selected="true"]')return state.drawer?new QaElement(state.section):null;
      if(selector==='.session-tab')return state.drawer?tabs[0]:null;
      if(selector==='.session-panel-close')return state.drawer?close:null;
      if(selector==='.session-card-open')return sessionControls[0]??null;
      if(selector==='#session-panel'){const panel=new QaElement('panel');panel.dataset={session:state.opened,sessionView:state.drawer?'detail':'list'};return panel;}
      if(selector==='.process-tree-view')return state.drawer&&state.section==='Process tree'?new QaElement('tree view'):null;
      if(selector==='.logical-panel')return state.clock >= (options.navigationDelay ?? 0) ? new QaElement(options.logicalText??'') : null;
      if(selector==='.activity-overview')return new QaElement(options.activityText??'');
      if(selector==='.session-process-list'){const list=new QaElement('');list.dataset.processView=state.mode;return list;}
      if(selector==='.session-process-card'){return table;}
      if(selector==='.session-process-grid'||selector==='.context-resource-grid')return table;
      if(selector==='.history-grid'||selector==='.context-history-grid')return history;
      if(selector==='.activity-grid')return history;
      if(selector==='.sidebar')return sidebar;
      if(selector==='.live-state')return new QaElement(state.live);
      if(selector==='.main-footer > span:last-child')return new QaElement(state.samples+' samples · 2s collection target');
      if(selector==='.resource-chart')return canvas;
      if(selector==='[data-native-failure]'&&options.nativeFailure){const failure=new QaElement('failure');failure.attributes['data-native-failure']='fixture native timeout';return failure;}
      return null;
    }};
  const context=vm.createContext({document,window:{Chart:{getChart:/** @returns {{options: {scales: {y: {title: {text: string}}}}}} */ ()=>({options:{scales:{y:{title:{text:'% of one core'}}}}})}},
    innerWidth:options.width??1240,innerHeight:options.height??850,
    getComputedStyle:/** @returns {{gridTemplateColumns: string}} */ ()=>({gridTemplateColumns:Array.from({length:state.columns},/** @returns {string} */ ()=> '400px').join(' ')}),
    performance:{now:/** @returns {number} */ ()=>state.clock},Event,
    setTimeout:/** @param {() => void} callback @param {number} milliseconds @returns {number} */ (callback,milliseconds)=>{queueMicrotask(/** @returns {void} */ ()=>{state.clock+=milliseconds;if(options.advanceSamples&&(state.live!=='Paused'||options.advancePaused))state.samples++;state.groupPending=false;if(options.replaceOnLayout&&!state.groupDropped){state.groupPending=true;state.groupDropped=true;}callback();});return 1;},
    requestAnimationFrame:/** @param {() => void} callback @returns {number} */ callback=>{queueMicrotask(callback);return 1;}});
  const source=fs.readFileSync(new URL('../../tools/native/NativePreviewReport.js',import.meta.url),'utf8');
  /** @type {QaReporter} */
  const report=vm.runInContext(source+'\nNativePreviewReport;',context);
  return {report,state,body,graphCanvases,scrolls,graphs,context};
}

/** @param {string} [action] @param {string} [check] @param {number} [width] @param {number} [height] @returns {QaInput} */
function input(action='tree',check='tree',width=1240,height=850) {
  return {label:'fixture-'+action,action,check,section:'Process tree',mode:'tree',width,height};
}

/** @param {QaEnvironment} context @returns {Promise<string>} */
async function tree(context) {return context.report.verify(input('tree','tree'),true);}

/** @param {string} content @returns {QaResult} */
function parse(content) { return JSON.parse(content); }

/** @param {QaResult} report @returns {QaInteraction} */
function interaction(report) {
  assert.ok(report.interaction);
  return report.interaction;
}

/** @param {QaResult} report @returns {QaOffset} */
function scroll(report) {
  const offset = interaction(report).scroll;
  assert.ok(offset);
  return offset;
}

test('one fresh reported agent with a source and assigned task satisfies roster verification',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalRows:[{id:'own-task',name:'/root',status:'running',source:'Coordinator-reported roster · current receipt',task:'Verify native app'}]});
  const request={...input(),action:'running',section:"What's running",mode:'',check:'logical'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
  assert.equal(report.logicalRecords.length,1);
  assert.equal(report.logicalRecords[0].task,'Verify native app');
});

test('a recovered stale agent does not satisfy fresh roster verification',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalRows:[{id:'old-task',name:'/root',status:'stale',source:'Recovered local roster',task:'Older task'}]});
  const request={...input(),action:'running',section:"What's running",mode:'',check:'logical'};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('missing roster provenance does not satisfy fresh roster verification',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalRows:[{id:'own-task',name:'/root',status:'running',source:'',task:'Verify native app'}]});
  const request={...input(),action:'running',section:"What's running",mode:'',check:'logical'};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('zoom verification waits through a poll replacing the current graph',/** @returns {Promise<void>} */ async()=>{
  const context=environment({replaceOnLayout:true,advanceSamples:true});
  await tree(context);
  const report=parse(await context.report.verify(input('exercise','zoom'),true));
  assert.equal(report.verification.success,true);
  assert.equal(interaction(report).baseline.after[0].scale,'scale(1.1)');
  assert.equal(interaction(report).baseline.after[1].scale,'scale(1)');
});

test('report opens the session details and waits for the Process tree tab before recording 1240x850',/** @returns {Promise<void>} */ async()=>{
  const context=environment();
  const report=parse(await context.report.verify(input(),true));
  assert.equal(report.section,'Process tree');
  assert.equal(context.state.drawer,true);
  assert.equal(report.width,1240);
  assert.equal(report.height,850);
  assert.equal(report.verification.success,true);
});

test('small tree records the actual 1000x700 viewport',/** @returns {Promise<void>} */ async()=>{
  const context=environment({width:1000,height:700,columns:1});
  const report=parse(await context.report.verify(input('tree','tree',1000,700),true));
  assert.equal(report.width,1000);
  assert.equal(report.height,700);
  assert.equal(report.section,'Process tree');
});

for(const action of ['table','owned-table','runtimes']) {
  test('removed '+action+' step fails with an explicit note',/** @returns {Promise<void>} */ async()=>{
    const context=environment();
    await assert.rejects(context.report.verify(input(action,''),true),/were removed; use the session details Process tree or Resources tab/);
  });
}

test('Activity tab opens from the session card holding the owned agent row and reports its section',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalRows:[{id:'other',name:'Other',status:'running',source:'Fixture',task:'Other task'},{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'Root',status:'running',source:'Fixture',task:'Root task'}]});
  const request={label:'activity',action:'activity',section:'Activity',mode:'',check:'',width:1240,height:850,agentRecordId:'coord:00000000-0000-4000-8000-000000000001:/root'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
  assert.equal(report.section,'Activity');
  assert.equal(context.state.opened,'coord:00000000-0000-4000-8000-000000000001:/root');
});

test("returning to What's running closes the session details",/** @returns {Promise<void>} */ async()=>{
  const context=environment();
  await tree(context);
  assert.equal(context.state.drawer,true);
  const request={...input(),action:'running',section:"What's running",mode:'',check:''};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.section,"What's running");
  assert.equal(context.state.drawer,false);
});

test('wrong late viewport times out rather than accepting a mislabeled small report',/** @returns {Promise<void>} */ async()=>{
  const context=environment();
  await assert.rejects(context.report.verify(input('tree','tree',1000,700),true),/timed out/);
});

test('native runtime error fails the current action',/** @returns {Promise<void>} */ async()=>{
  const context=environment({nativeFailure:true});
  await assert.rejects(context.report.verify(input(),true),/fixture native timeout/);
});

test('missing process groups do not produce empty successful zoom comparisons',/** @returns {Promise<void>} */ async()=>{
  const context=environment({noGroups:true});
  await assert.rejects(context.report.verify(input('exercise','zoom'),true),/needs two actual process groups/);
});

test('independent zoom and group pan survive three externally observed sample updates',/** @returns {Promise<void>} */ async()=>{
  const context=environment({advanceSamples:true});
  await tree(context);
  const first=parse(await context.report.verify(input('exercise','zoom'),true));
  const next=parse(await context.report.verify(input('polls','polls'),true));
  assert.equal(interaction(first).baseline.before[0].scale,'scale(1)');
  assert.equal(interaction(first).baseline.after[0].scale,'scale(1.1)');
  assert.equal(interaction(first).baseline.after[1].scale,'scale(1)');
  assert.ok(next.sampleCount>=first.sampleCount+3);
  assert.equal(scroll(next).top,120);
});

test('live grid reordering preserves verification of the original groups and scrolled selection', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  await tree(context);
  const baseline=parse(await context.report.verify(input('exercise','zoom'),true));
  context.graphs.reverse();
  const report=parse(await context.report.verify(input('polls','polls'),true));
  assert.deepEqual(interaction(report).current,interaction(baseline).baseline.after);
  assert.deepEqual(interaction(report).scroll,interaction(baseline).baseline.scroll);
  assert.equal(context.graphs[0].dataset.group,'fixture-b');
});

test('missing original group fails instead of verifying a replacement position', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  await tree(context);
  await context.report.verify(input('exercise','zoom'),true);
  context.graphs.shift();
  await assert.rejects(context.report.verify(input('polls','polls'),true),/process group fixture-a/);
});

test('changed birth-keyed group identity cannot substitute for the baseline group', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  context.graphs[0].dataset.group='codex:41:100:7';
  context.scrolls[0].dataset.group='codex:41:100:7';
  await tree(context);
  await context.report.verify(input('exercise','zoom'),true);
  context.graphs[0].dataset.group='codex:41:101:7';
  context.scrolls[0].dataset.group='codex:41:101:7';
  await assert.rejects(context.report.verify(input('polls','polls'),true),/process group codex:41:100:7/);
});

test('lost original group zoom still fails after grid reordering', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  await tree(context);
  await context.report.verify(input('exercise','zoom'),true);
  context.graphs.reverse();
  zoomCanvas(context.graphCanvases[0],1);
  await assert.rejects(context.report.verify(input('polls','polls'),true),/independent zoom or zoom preservation/);
});

test('other original group zoom change still fails after grid reordering', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  await tree(context);
  await context.report.verify(input('exercise','zoom'),true);
  context.graphs.reverse();
  zoomCanvas(context.graphCanvases[1],1.2);
  await assert.rejects(context.report.verify(input('polls','polls'),true),/independent zoom or zoom preservation/);
});

test('lost original group pan still fails when zoom is preserved', /** @returns {Promise<void>} */ async () => {
  const context=environment({advanceSamples:true});
  await tree(context);
  await context.report.verify(input('exercise','zoom'),true);
  context.scrolls[0].scrollTop=0;
  await assert.rejects(context.report.verify(input('polls','polls'),true),/group pan changed/);
});

for(const action of ['pause','resume','hold']) {
  test('unsupported '+action+' fails without claiming a native control exists',/** @returns {Promise<void>} */ async()=>{
    const context=environment({advanceSamples:true});
    await assert.rejects(context.report.verify(input(action,action),true),/Unsupported native QA action/);
  });
}

test('Resources report records real DOM chart values in one full-width column',/** @returns {Promise<void>} */ async()=>{
  const context=environment({columns:1});
  const request={...input(),label:'fixture-history',action:'history',section:'Resources',mode:'',check:'history'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.section,'Resources');
  assert.equal(report.chartPoints,12);
  assert.equal(report.chartUnit,'% of one core');
  assert.equal(report.historyLayout.columns.length,1);
});

test('Resources grid with two columns fails inside the drawer',/** @returns {Promise<void>} */ async()=>{
  const context=environment({columns:2});
  const request={...input(),label:'fixture-history',action:'history',section:'Resources',mode:'',check:'history'};
  await assert.rejects(context.report.verify(request,true),/expected 1 equal columns; measured 2/);
});


test('native startup waits for asynchronous navigation before pressing a control', /** @returns {Promise<void>} */ async () => {
  const context = environment({navigationDelay: 400});
  const report = parse(await context.report.verify(input(), true));
  assert.equal(report.verification.success, true);
  assert.ok(context.state.clock >= 400);
});


test('agent capture observes live updates without requiring process tree zoom state', /** @returns {Promise<void>} */ async () => {
  const context = environment({advanceSamples:true});
  const request = {...input(), action:'running', section:"What's running", mode:'', check:'updates'};
  const report = parse(await context.report.verify(request,true));
  assert.equal(report.section,"What's running");
  assert.ok(report.sampleCount >= 13);
  assert.equal(report.interaction,null);
  assert.equal(report.verification.success,true);
});

test('capture follows the actual user window size while fixed viewport checks stay strict', /** @returns {Promise<void>} */ async () => {
  const context = environment({advanceSamples:true,width:1920,height:928});
  const request = {...input(), action:'running', section:"What's running", mode:'', check:'updates'};
  const report = parse(await context.report.verify(request,true));
  assert.equal(report.width,1920);
  assert.equal(report.height,928);
  assert.equal(report.verification.success,true);
});

test('current-work check accepts the verified session-qualified coordinator ID', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'/root',status:'running',source:'Coordinator-reported team roster',task:'fixture task',action:'current_validation'}]});
  const request={...focusedInput('current-work-live'),section:"What's running",mode:'',actionName:'current_validation'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
  assert.equal(report.logicalRecords[0].id,'coord:00000000-0000-4000-8000-000000000001:/root');
});

test('current-work check does not claim an unqualified same-name coordinator', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'/root',name:'/root',status:'running',source:'Coordinator-reported team roster',task:'fixture task',action:'current_validation'}]});
  const request={...focusedInput('current-work-hidden'),section:"What's running",mode:'',actionName:'current_validation'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
});

test('task-only check accepts an active assignment without command fields', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'/root',status:'active',source:'Coordinator-reported team roster',task:'fixture task'}]});
  const request={...focusedInput('current-task-only'),section:"What's running",mode:''};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
  assert.equal(report.logicalRecords[0].status,'active');
});

test('task-only check rejects a row without an assigned task', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'/root',status:'active',source:'Coordinator-reported team roster',task:''}]});
  const request={...focusedInput('current-task-only'),section:"What's running",mode:''};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('task-only check rejects a card still showing a command', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'/root',status:'running',source:'Coordinator-reported team roster',task:'fixture task',action:'reported_action'}]});
  const request={...focusedInput('current-task-only'),section:"What's running",mode:''};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('current-work verifier matches explicitly supplied session-qualified identity', /** @returns {Promise<void>} */ async () => {
  const explicit='coord:00000000-0000-4000-8000-000000000002:/root/fixture_worker';
  const context=environment({logicalRows:[{id:explicit,sessionId:'00000000-0000-4000-8000-000000000002',name:'Fixture worker',status:'running',source:'Coordinator-reported team roster',task:'fixture task',action:'fixture_action'}]});
  const request={...focusedInput('current-work-live'),section:"What's running",mode:'',actionName:'fixture_action',agentRecordId:explicit,sessionId:'00000000-0000-4000-8000-000000000002'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
  assert.equal(report.logicalRecords[0].id,explicit);
});

test('current-work verifier refuses an unconfigured coordinator identity', /** @returns {Promise<void>} */ async () => {
  const context=environment({logicalRows:[{id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'Fixture worker',status:'running',source:'Coordinator-reported team roster',task:'fixture task',action:'fixture_action'}]});
  delete context.body.dataset.qaAgentId;
  const request={...input('running','current-work-live'),section:"What's running",mode:'',actionName:'fixture_action'};
  delete request.agentRecordId;
  await assert.rejects(context.report.verify(request,true), /identity|timed out/);
});

/** @param {string} check @returns {QaInput} */
function focusedInput(check) {
  return {...input('running',check),section:"What's running",mode:'',agentRecordId:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',actionRecordId:'fixture-action',actionName:'fixture_action'};
}

for(const changed of /** @type {('agent'|'sessionId'|'taskId'|'id'|'source'|'status')[]} */ (['agent','sessionId','taskId','id','source','status'])) {
  test('completed history cannot be satisfied by a different '+changed,/** @returns {Promise<void>} */ async()=>{
    const action={id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'fixture_action',status:'completed',source:'Coordinator-reported team roster',running:false};
    action[changed]='other-fixture';
    const context=environment({actions:[action]});
    const request={...focusedInput('current-work-completed'),action:'activity',section:'Activity'};
    await assert.rejects(context.report.verify(request,true),/timed out/);
  });
}

test('completed history accepts only the caller-supplied agent session task and execution',/** @returns {Promise<void>} */ async()=>{
  const context=environment({actions:[{id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'fixture_action',status:'completed',source:'Coordinator-reported team roster',running:false}]});
  const request={...focusedInput('current-work-completed'),action:'activity',section:'Activity'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
});

for(const changed of /** @type {('sessionId'|'taskId'|'rawSource')[]} */ (['sessionId','taskId','rawSource'])) {
  test('task-only report refuses mismatched '+changed,/** @returns {Promise<void>} */ async()=>{
    const record={id:'coord:00000000-0000-4000-8000-000000000001:/root',name:'Fixture agent',status:'active',source:'Coordinator-reported team roster',task:'Fixture task',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',rawSource:'Coordinator-reported team roster'};
    record[changed]='';
    const context=environment({logicalRows:[record]});
    await assert.rejects(context.report.verify(focusedInput('current-task-only'),true),/timed out/);
  });
}

test('unfinished historical running action is not a current running execution',/** @returns {Promise<void>} */ async()=>{
  const context=environment({actions:[{id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'fixture_action',status:'running',source:'Coordinator-reported team roster',running:false}]});
  const request={...focusedInput('action-running'),action:'activity',section:'Activity'};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('verified current running action records exact ownership',/** @returns {Promise<void>} */ async()=>{
  const context=environment({actions:[{id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'fixture_action',status:'running',source:'Coordinator-reported team roster',running:true}]});
  const request={...focusedInput('action-running'),action:'activity',section:'Activity'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
});

test('pan verification requires a real overflowing process viewport',/** @returns {Promise<void>} */ async()=>{
  const context=environment({overflowScroll:false,advanceSamples:true});
  await tree(context);
  await assert.rejects(context.report.verify(input('exercise','zoom'),true),/overflowing real process tree/);
});

test('hidden owned task does not reject another caller label containing stale',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalText:'Another active task: stale_cleanup',logicalRows:[{id:'other-fixture',name:'Other fixture',status:'active',source:'Coordinator-reported team roster',task:'stale_cleanup'}]});
  const report=parse(await context.report.verify(focusedInput('current-work-hidden'),true));
  assert.equal(report.verification.success,true);
});

test('genuine completed stale_cleanup label is preserved without a false freshness failure',/** @returns {Promise<void>} */ async()=>{
  const context=environment({activityText:'Retained action: stale_cleanup',actions:[{id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'stale_cleanup',status:'completed',source:'Coordinator-reported team roster',running:false}]});
  const request={...focusedInput('current-work-completed'),action:'activity',section:'Activity',actionName:'stale_cleanup'};
  const report=parse(await context.report.verify(request,true));
  assert.equal(report.verification.success,true);
});

test('actual stale action status still cannot prove completion',/** @returns {Promise<void>} */ async()=>{
  const context=environment({actions:[{id:'fixture-action',agent:'coord:00000000-0000-4000-8000-000000000001:/root',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',name:'fixture_action',status:'stale',source:'Coordinator-reported team roster',running:false}]});
  const request={...focusedInput('current-work-completed'),action:'activity',section:'Activity'};
  await assert.rejects(context.report.verify(request,true),/timed out/);
});

test('open-first-session clicks the first session card control and reports the panel state',/** @returns {Promise<void>} */ async()=>{
  const context=environment({logicalRows:[{id:'first-agent',name:'First session',status:'running',source:'Fixture',task:'First task'},{id:'second-agent',name:'Second session',status:'running',source:'Fixture',task:'Second task'}]});
  const request={label:'native-session-open',action:'open-first-session',section:'Resources',mode:'',check:'',width:1240,height:850};
  const report=JSON.parse(await context.report.verify(request,true));
  assert.equal(context.state.drawer,true);
  assert.equal(context.state.opened,'first-agent');
  assert.equal(report.diagnostics.panel.session,'first-agent');
  assert.equal(report.diagnostics.panel.view,'detail');
  assert.equal('drawer' in report.diagnostics,false);
});
