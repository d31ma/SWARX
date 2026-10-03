import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {test} from 'node:test';
import {QaPathUtils} from './QaPathUtils.mjs';

class NativeLiveQaReportTest {
  /** @type {string} */
  static UTILITY = new URL('./NativeLiveQaReportUtils.mjs',import.meta.url).href;
  /** @type {{agentRecordId: string, sessionId: string, taskId: string, actionRecordId: string, actionName: string}} */
  static OWNER = {agentRecordId:'coord:00000000-0000-4000-8000-000000000001:/root/fixture_worker',sessionId:'00000000-0000-4000-8000-000000000001',taskId:'fixture-task',actionRecordId:'fixture-action',actionName:'fixture_action'};

  /** @param {import('node:test').TestContext} context @returns {string} */
  static directory(context) {
    const directory=fs.mkdtempSync(path.join(QaPathUtils.external('OBSERVATORY_QA_ROOT'),'native-report-fixture-'));
    context.after(/** @returns {void} */()=>fs.rmSync(directory,{recursive:true,force:true}));
    return directory;
  }

  /** @param {string} directory @param {string} filename @param {unknown} report @returns {string} */
  static file(directory,filename,report) {
    const file=path.join(directory,filename);
    fs.writeFileSync(file,JSON.stringify(report));
    return file;
  }

  /** @param {string} file @returns {import('node:child_process').SpawnSyncReturns<string>} */
  static stage(file) {
    const program='import {NativeLiveQaReportUtils} from '+JSON.stringify(this.UTILITY)+';NativeLiveQaReportUtils.stage('+JSON.stringify(file)+','+JSON.stringify(this.OWNER)+');';
    return spawnSync(process.execPath,['--input-type=module','--eval',program],{encoding:'utf8'});
  }

  /** @param {string} file @param {string} directory @returns {import('node:child_process').SpawnSyncReturns<string>} */
  static status(file,directory) {
    const program='import {NativeLiveQaReportUtils} from '+JSON.stringify(this.UTILITY)+';NativeLiveQaReportUtils.status('+JSON.stringify(file)+','+JSON.stringify(directory)+',7);';
    return spawnSync(process.execPath,['--input-type=module','--eval',program],{encoding:'utf8'});
  }

  /** @param {string} check @param {string} label @returns {{capturedAt: string, section: string, horizontalOverflow: boolean, documentScroll: boolean, sidebarOverflow: boolean, appIconLoaded: boolean, verification: {success: boolean, label: string, requested: Record<string,string>}, logicalRecords: Record<string,unknown>[], actionHistory: Record<string,unknown>[]}} */
  static report(check='current-task-only',label='native-task-only') {
    return {capturedAt:'2026-01-01T12:00:00.000Z',section:check==='current-work-completed'?'Activity':"What's running",horizontalOverflow:false,documentScroll:false,sidebarOverflow:false,appIconLoaded:true,
      verification:{success:true,label,requested:{label,check,...this.OWNER}},
      logicalRecords:[{id:this.OWNER.agentRecordId,sessionId:this.OWNER.sessionId,taskId:this.OWNER.taskId,status:'active',source:'Coordinator-reported team roster',task:'Fixture task',currentAction:'',commandResourceCount:0}],
      actionHistory:[{id:this.OWNER.actionRecordId,agent:this.OWNER.agentRecordId,sessionId:this.OWNER.sessionId,taskId:this.OWNER.taskId,name:this.OWNER.actionName,status:'completed',source:'Coordinator-reported team roster',running:false,text:'Measured by the reporting caller'}]};
  }
}

test('external JSON stage boundary accepts an exact task-only report',/** @param {import('node:test').TestContext} context @returns {void} */context=>{
  const directory=NativeLiveQaReportTest.directory(context);
  const file=NativeLiveQaReportTest.file(directory,'native-task-only-report.json',NativeLiveQaReportTest.report());
  const result=NativeLiveQaReportTest.stage(file);
  assert.equal(result.status,0,result.stderr);
});

for(const field of ['agentRecordId','sessionId','taskId','actionRecordId']) {
  test('external stage boundary refuses a requested different '+field,/** @param {import('node:test').TestContext} context @returns {void} */context=>{
    const directory=NativeLiveQaReportTest.directory(context);
    const report=NativeLiveQaReportTest.report();
    report.verification.requested[field]='other-fixture';
    const result=NativeLiveQaReportTest.stage(NativeLiveQaReportTest.file(directory,'native-task-only-report.json',report));
    assert.notEqual(result.status,0);
    assert.match(result.stderr,/requested identity/);
  });
}

for(const field of ['id','sessionId','taskId','source','status','currentAction','commandResourceCount']) {
  test('external stage boundary refuses invalid task-only '+field,/** @param {import('node:test').TestContext} context @returns {void} */context=>{
    const directory=NativeLiveQaReportTest.directory(context);
    const report=NativeLiveQaReportTest.report();
    report.logicalRecords[0][field]='other-fixture';
    const result=NativeLiveQaReportTest.stage(NativeLiveQaReportTest.file(directory,'native-task-only-report.json',report));
    assert.notEqual(result.status,0);
    assert.match(result.stderr,/task-only/);
  });
}

test('external JSON completion boundary accepts the exact retained action owner',/** @param {import('node:test').TestContext} context @returns {void} */context=>{
  const directory=NativeLiveQaReportTest.directory(context);
  const report=NativeLiveQaReportTest.report('current-work-completed','native-task-completed-history');
  const result=NativeLiveQaReportTest.stage(NativeLiveQaReportTest.file(directory,'native-task-completed-history-report.json',report));
  assert.equal(result.status,0,result.stderr);
});

for(const field of ['id','agent','sessionId','taskId','source','name','status']) {
  test('external JSON completion boundary rejects a different '+field,/** @param {import('node:test').TestContext} context @returns {void} */context=>{
    const directory=NativeLiveQaReportTest.directory(context);
    const report=NativeLiveQaReportTest.report('current-work-completed','native-task-completed-history');
    report.actionHistory[0][field]='other-fixture';
    const result=NativeLiveQaReportTest.stage(NativeLiveQaReportTest.file(directory,'native-task-completed-history-report.json',report));
    assert.notEqual(result.status,0);
    assert.match(result.stderr,/completed action/);
  });
}

test('status boundary requires the same directory as observed stage reports',/** @param {import('node:test').TestContext} context @returns {void} */context=>{
  const directory=NativeLiveQaReportTest.directory(context);
  const file=NativeLiveQaReportTest.file(directory,'status.json',{success:true,directory,completedSteps:7});
  assert.equal(NativeLiveQaReportTest.status(file,directory).status,0);
  assert.notEqual(NativeLiveQaReportTest.status(file,path.join(directory,'other-fixture')).status,0);
});

test('status boundary rejects a failed run or incomplete stage count',/** @param {import('node:test').TestContext} context @returns {void} */context=>{
  const directory=NativeLiveQaReportTest.directory(context);
  const failed=NativeLiveQaReportTest.file(directory,'failed.json',{success:false,directory,completedSteps:7});
  const incomplete=NativeLiveQaReportTest.file(directory,'incomplete.json',{success:true,directory,completedSteps:1});
  assert.notEqual(NativeLiveQaReportTest.status(failed,directory).status,0);
  assert.notEqual(NativeLiveQaReportTest.status(incomplete,directory).status,0);
});

test('stage discovery rejects two new directories instead of choosing another run',/** @param {import('node:test').TestContext} context @returns {void} */context=>{
  const directory=NativeLiveQaReportTest.directory(context);
  fs.mkdirSync(path.join(directory,'native-qa-fixture-a'));
  fs.mkdirSync(path.join(directory,'native-qa-fixture-b'));
  fs.writeFileSync(path.join(directory,'native-qa-fixture-a','native-task-only-report.json'),'{}');
  fs.writeFileSync(path.join(directory,'native-qa-fixture-b','native-task-only-report.json'),'{}');
  const program='import {NativeLiveQaReportUtils} from '+JSON.stringify(NativeLiveQaReportTest.UTILITY)+';NativeLiveQaReportUtils.findStage('+JSON.stringify(directory)+',new Set(),undefined,"native-task-only-report.json");';
  const result=spawnSync(process.execPath,['--input-type=module','--eval',program],{encoding:'utf8'});
  assert.notEqual(result.status,0);
  assert.match(result.stderr,/multiple/);
});
