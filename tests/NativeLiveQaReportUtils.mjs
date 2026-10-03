import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

/** @typedef {{agentRecordId: string, sessionId: string, taskId: string, actionRecordId: string, actionName: string, startedAt?: number}} QaReportOwner */
/** @typedef {{label: string, check: string, section: string}} QaReportStage */
/** @typedef {{id?: string, sessionId?: string, taskId?: string, status?: string, source?: string, task?: string, currentAction?: string, commandResourceCount?: number}} QaLogicalStageRecord */
/** @typedef {{id?: string, agent?: string, sessionId?: string, taskId?: string, source?: string, name?: string, status?: string, running?: boolean, text?: string}} QaActionStageRecord */
/** @typedef {{capturedAt?: string, section?: string, horizontalOverflow?: boolean, documentScroll?: boolean, sidebarOverflow?: boolean, appIconLoaded?: boolean, verification?: {success?: boolean, label?: string, requested?: {label?: string, check?: string, agentRecordId?: string, sessionId?: string, taskId?: string, actionRecordId?: string, actionName?: string}}, logicalRecords?: QaLogicalStageRecord[], actionHistory?: QaActionStageRecord[]}} QaStageReport */
/** @typedef {{success?: boolean, directory?: string, completedSteps?: number}} QaStatusReport */

export class NativeLiveQaReportUtils {
  /** @type {string} */
  static SOURCE = 'Coordinator-reported team roster';
  /** @type {Record<string, QaReportStage>} */
  static STAGES = {
    'native-task-only-report.json':{label:'native-task-only',check:'current-task-only',section:"What's running"},
    'native-task-only-polls-report.json':{label:'native-task-only-polls',check:'current-task-polls',section:"What's running"},
    'native-task-only-small-report.json':{label:'native-task-only-small',check:'current-task-only',section:"What's running"},
    'native-task-command-report.json':{label:'native-task-command',check:'current-work-live',section:"What's running"},
    'native-task-between-commands-report.json':{label:'native-task-between-commands',check:'current-task-only',section:"What's running"},
    'native-task-completed-history-report.json':{label:'native-task-completed-history',check:'current-work-completed',section:'Activity'},
    'native-task-completed-hidden-report.json':{label:'native-task-completed-hidden',check:'current-work-hidden',section:"What's running"},
    'native-current-live-report.json':{label:'native-current-live',check:'current-work-live',section:"What's running"},
    'native-current-expired-report.json':{label:'native-current-expired',check:'current-work-hidden',section:"What's running"},
    'native-current-reappeared-report.json':{label:'native-current-reappeared',check:'current-work-live',section:"What's running"},
    'native-current-completed-history-report.json':{label:'native-current-completed-history',check:'current-work-completed',section:'Activity'},
    'native-current-completed-hidden-report.json':{label:'native-current-completed-hidden',check:'current-work-hidden',section:"What's running"}
  };

  /** @param {string} file @param {QaReportOwner} owner @returns {QaStageReport} */
  static stage(file,owner) {
    const stage=this.#stage(path.basename(file));
    const report=/** @type {QaStageReport} */ (this.#read(file));
    this.#requireReport(report,stage,owner);
    this.#requireView(report,stage,owner);
    return report;
  }

  /** @param {string} file @param {string} directory @param {number} steps @returns {QaStatusReport} */
  static status(file,directory,steps) {
    const report=/** @type {QaStatusReport} */ (this.#read(file));
    assert.equal(report.success,true,'Native run did not succeed');
    assert.equal(report.directory,directory,'Native status directory does not match observed stages');
    assert.equal(report.completedSteps,steps,'Native run did not complete every required stage');
    return report;
  }

  /** @param {string} evidence @param {Set<string>} previous @param {string | undefined} selected @param {string} filename @returns {string | undefined} */
  static findStage(evidence,previous,selected,filename) {
    const directories=this.#directories(evidence,previous,selected);
    const matches=directories.map(/** @param {string} directory @returns {string} */directory=>path.join(directory,filename)).filter(fs.existsSync);
    assert.ok(matches.length<=1,'Native stage found multiple new QA directories');
    return matches[0];
  }

  /** @param {string} file @returns {unknown} */
  static #read(file) {
    return JSON.parse(fs.readFileSync(file,'utf8'));
  }

  /** @param {string} filename @returns {QaReportStage} */
  static #stage(filename) {
    const stage=this.STAGES[filename];
    assert.ok(stage,'Unknown native report stage: '+filename);
    return stage;
  }

  /** @param {QaStageReport} report @param {QaReportStage} stage @param {QaReportOwner} owner @returns {void} */
  static #requireReport(report,stage,owner) {
    const captured=Date.parse(report.capturedAt??'');
    assert.ok(Number.isFinite(captured),'Native stage has no valid capture time');
    assert.ok(captured>=(owner.startedAt??0),'Native stage predates this run');
    assert.equal(report.verification?.success,true,'Native stage verification did not succeed');
    assert.equal(report.verification?.label,stage.label,'Native stage label differs');
    assert.equal(report.verification?.requested?.label,stage.label,'Native requested label differs');
    assert.equal(report.verification?.requested?.check,stage.check,'Native requested check differs');
    assert.equal(report.section,stage.section,'Native stage view differs');
    this.#requireRequested(report.verification?.requested,owner);
    assert.equal(report.horizontalOverflow,false,'Native page overflows horizontally');
    assert.equal(report.documentScroll,false,'Native page scrolls outside its shell');
    assert.equal(report.sidebarOverflow,false,'Native sidebar overflows');
    assert.equal(report.appIconLoaded,true,'Native application icon is unavailable');
    assert.ok(Array.isArray(report.logicalRecords),'Native stage omits logical records');
    assert.ok(Array.isArray(report.actionHistory),'Native stage omits action records');
  }

  /** @param {NonNullable<QaStageReport['verification']>['requested']} requested @param {QaReportOwner} owner @returns {void} */
  static #requireRequested(requested,owner) {
    assert.equal(requested?.agentRecordId,owner.agentRecordId,'Native requested identity differs: agent');
    assert.equal(requested?.sessionId,owner.sessionId,'Native requested identity differs: session');
    assert.equal(requested?.taskId,owner.taskId,'Native requested identity differs: task');
    assert.equal(requested?.actionRecordId,owner.actionRecordId,'Native requested identity differs: action');
    assert.equal(requested?.actionName,owner.actionName,'Native requested action name differs');
  }

  /** @param {QaStageReport} report @param {QaReportStage} stage @param {QaReportOwner} owner @returns {void} */
  static #requireView(report,stage,owner) {
    const logical=report.logicalRecords??[];
    const actions=report.actionHistory??[];
    switch(stage.check) {
      case 'current-work-hidden': assert.ok(!logical.some(/** @param {QaLogicalStageRecord} record @returns {boolean} */record=>record.id===owner.agentRecordId),'Completed or expired agent remains in Running');return;
      case 'current-work-completed': assert.ok(actions.some(/** @param {QaActionStageRecord} record @returns {boolean} */record=>this.#completed(record,owner)),'Native completed action differs from the configured owner');return;
      case 'current-work-live': assert.ok(logical.some(/** @param {QaLogicalStageRecord} record @returns {boolean} */record=>[this.#activeTask(record,owner),record.currentAction===owner.actionName].every(Boolean)),'Native current action differs from the configured owner');return;
      default: assert.ok(logical.some(/** @param {QaLogicalStageRecord} record @returns {boolean} */record=>[this.#activeTask(record,owner),record.currentAction==='',record.commandResourceCount===0].every(Boolean)),'Native task-only report differs from the configured owner');return;
    }
  }

  /** @param {QaLogicalStageRecord} record @param {QaReportOwner} owner @returns {boolean} */
  static #activeTask(record,owner) {
    return [record.id===owner.agentRecordId,record.sessionId===owner.sessionId,record.taskId===owner.taskId,
      record.source===this.SOURCE,['active','running'].includes(record.status??''),Boolean(record.task?.trim())].every(Boolean);
  }

  /** @param {QaActionStageRecord} record @param {QaReportOwner} owner @returns {boolean} */
  static #completed(record,owner) {
    return [record.id===owner.actionRecordId,record.agent===owner.agentRecordId,
      record.sessionId===owner.sessionId,record.taskId===owner.taskId,record.source===this.SOURCE,
      record.name===owner.actionName,record.status==='completed',record.running===false,
      Boolean(record.text?.includes('Measured by the reporting caller'))].every(Boolean);
  }

  /** @param {string} evidence @param {Set<string>} previous @param {string | undefined} selected @returns {string[]} */
  static #directories(evidence,previous,selected) {
    if(selected!==undefined)return [selected];
    return fs.readdirSync(evidence,{withFileTypes:true}).filter(/** @param {import('node:fs').Dirent} entry @returns {boolean} */entry=>[entry.isDirectory(),entry.name.startsWith('native-qa-'),!previous.has(entry.name)].every(Boolean)).map(/** @param {import('node:fs').Dirent} entry @returns {string} */entry=>path.join(evidence,entry.name));
  }
}
