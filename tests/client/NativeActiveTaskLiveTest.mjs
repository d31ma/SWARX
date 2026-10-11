import {userInfo} from 'node:os';
import {QaPathUtils} from './QaPathUtils.mjs';
import {NativeLiveQaReportUtils} from './NativeLiveQaReportUtils.mjs';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn, spawnSync} from 'node:child_process';

class NativeActiveTaskLiveTest {
  /** @type {string} */
  static ROOT = QaPathUtils.external('OBSERVATORY_QA_ROOT');
  /** @type {string} */
  static BUNDLE = QaPathUtils.external('OBSERVATORY_APP_BUNDLE');
  /** @type {string} */
  static SOCKET = '/tmp/agent-observatory-' + userInfo().uid + '/hooks-native.sock';
  /** @type {string} */
  static STATUS = path.join(this.ROOT, 'evidence/native-qa-status.json');
  /** @type {string} */
  static AGENT = QaPathUtils.required('OBSERVATORY_QA_AGENT_ID');
  /** @type {string} */
  static SESSION = QaPathUtils.required('OBSERVATORY_QA_SESSION_ID');
  /** @type {string} */
  static SESSION_LABEL = QaPathUtils.required('OBSERVATORY_QA_SESSION_LABEL');
  /** @type {string} */
  static FAMILY = QaPathUtils.required('OBSERVATORY_QA_FAMILY');
  /** @type {string} */
  static TASK_NAME = QaPathUtils.required('OBSERVATORY_QA_TASK_NAME');
  /** @type {string} */
  static TASK = QaPathUtils.required('OBSERVATORY_QA_TASK_ID');
  /** @type {string} */
  static ACTION = QaPathUtils.required('OBSERVATORY_QA_ACTION_NAME');
  /** @type {string} */
  static EXECUTION = 'active-task-' + Date.now();
  /** @type {number} */
  static EVENT_TIMEOUT = 5000;
  /** @type {number} */
  static STAGE_TIMEOUT = 180000;
  /** @type {number} */
  static POLL_INTERVAL = 100;
  /** @type {{type: string, accepted: boolean, revision: number | null}[]} */
  static receipts = [];
  /** @type {Set<string>} */
  static previousReports = new Set();
  /** @type {number} */
  static EXIT_TIMEOUT = 10000;
  /** @type {string | undefined} */
  static reportDirectory;
  /** @type {number} */
  static startedAt = 0;

  /** @returns {Promise<void>} */
  static async run() {
    this.startedAt = Date.now();
    this.previousReports = new Set(fs.readdirSync(path.join(this.ROOT, 'evidence')));
    assert.equal(fs.existsSync(this.SOCKET), false, 'Quit the owned monitor before starting native QA.');
    fs.rmSync(this.STATUS, {force:true});
    const log = fs.openSync(path.join(this.ROOT,'evidence/active-task-native-app.txt'),'w');
    const app = spawn(path.join(this.BUNDLE,'Contents/MacOS/Swarx'),[],{
      env:{...process.env,OBSERVATORY_QA:'1',OBSERVATORY_QA_FOCUS:'active-task',
        OBSERVATORY_QA_ACTION_NAME:this.ACTION,OBSERVATORY_QA_ACTION_RECORD_ID:this.EXECUTION,OBSERVATORY_QA_AGENT_RECORD_ID:`coord:${this.SESSION}:${this.AGENT}`,OBSERVATORY_ROOT:this.ROOT,OBSERVATORY_LIFETIME_QA:''},
      stdio:['ignore',log,log]
    });
    fs.closeSync(log);
    const exit = new Promise(/** @param {(code:number|null)=>void} resolve @param {(error:Error)=>void} reject @returns {void} */ (resolve,reject)=>{
      app.once('error',reject);app.once('exit',resolve);
    });
    await this.waitPath(this.SOCKET);
    this.report('active');
    fs.writeFileSync(path.join(this.ROOT,'evidence/active-task-live-start.json'),JSON.stringify({
      started_at:new Date().toISOString(),app_pid:app.pid,session:this.SESSION,
      task:this.TASK,execution:this.EXECUTION,action:this.ACTION,receipts:this.receipts
    },null,2)+'\n');
    await this.waitStage('native-task-only-report.json');
    await this.waitStage('native-task-only-polls-report.json');
    await this.waitStage('native-task-only-small-report.json');
    const actionStarted=process.hrtime.bigint();
    this.event({type:'logical.coordination.execution.start',agent:this.AGENT,session_id:this.SESSION,
      session_name:this.SESSION_LABEL,task:this.TASK,execution:this.EXECUTION,name:this.ACTION});
    await this.waitStage('native-task-command-report.json');
    const duration=Number(process.hrtime.bigint()-actionStarted)/1e9;
    this.event({type:'logical.coordination.execution.finish',agent:this.AGENT,session_id:this.SESSION,
      task:this.TASK,execution:this.EXECUTION,status:'completed',exit_code:0,duration_seconds:duration});
    this.report('active');
    await this.waitStage('native-task-between-commands-report.json');
    this.report('completed');
    await this.waitStage('native-task-completed-history-report.json');
    await this.waitStage('native-task-completed-hidden-report.json');
    await this.waitPath(this.STATUS);
    const status=NativeLiveQaReportUtils.status(this.STATUS,this.reportDirectory??'',7);
    fs.writeFileSync(path.join(this.ROOT,'evidence/active-task-native-status.json'),JSON.stringify(status,null,2)+'\n');
    const quit=spawnSync('/usr/bin/osascript',['-e','tell application id "ma.del.agent-observatory" to quit'],{
      encoding:'utf8',timeout:this.EVENT_TIMEOUT
    });
    assert.equal(quit.status,0,quit.stderr);
    const code=await this.waitExit(exit);
    assert.equal(code,0);
    assert.equal(fs.existsSync(this.SOCKET),false,'Normal Quit must remove the private socket');
    fs.writeFileSync(path.join(this.ROOT,'evidence/active-task-live-result.json'),JSON.stringify({
      checked_at:new Date().toISOString(),success:true,app_pid:app.pid,session:this.SESSION,
      task:this.TASK,execution:this.EXECUTION,command_duration_seconds:duration,
      normal_quit_exit:code,socket_removed:true,receipts:this.receipts,
      scope:'Actual own native validation task and action reported through supported local IPC. Task-only, polls, small window, real action start/finish, between-command task and explicit task completion. No invented agents, process ownership or remote resource metrics.'
    },null,2)+'\n');
    console.log('PASS: real active task remains visible without commands and across polls, action fields follow actual execution, completed task stays in Activity; normal Quit.');
  }

  /** @param {string} status @returns {void} */
  static report(status) {
    this.event({type:'logical.coordination.upsert',agent:this.AGENT,session_id:this.SESSION,
      session_name:this.SESSION_LABEL,name:this.AGENT,family:this.FAMILY,status,task:this.TASK,
      task_name:this.TASK_NAME});
  }

  /** @param {{type:string,[key:string]:unknown}} event @returns {void} */
  static event(event) {
    const sent=spawnSync(path.join(this.BUNDLE,'Contents/MacOS/observatory-event'),[],{
      input:JSON.stringify(event),encoding:'utf8',timeout:this.EVENT_TIMEOUT
    });
    assert.equal(sent.status,0,sent.stderr+sent.stdout);
    const receipt=JSON.parse(sent.stdout);
    assert.equal(receipt.accepted,true,receipt.error);
    this.receipts.push({type:event.type,accepted:true,revision:receipt.revision});
  }

  /** @param {string} filename @returns {Promise<string>} */
  static async waitStage(filename) {
    const deadline=Date.now()+this.STAGE_TIMEOUT;
    for(;;) {
      this.requireNoFailure();
      const found=NativeLiveQaReportUtils.findStage(path.join(this.ROOT,'evidence'),this.previousReports,this.reportDirectory,filename);
      if(found) {
        NativeLiveQaReportUtils.stage(found,{agentRecordId:`coord:${this.SESSION}:${this.AGENT}`,sessionId:this.SESSION,taskId:this.TASK,actionRecordId:this.EXECUTION,actionName:this.ACTION,startedAt:this.startedAt});
        this.reportDirectory=path.dirname(found);
        return found;
      }
      assert.ok(Date.now()<deadline,'Native stage did not finish: '+filename);
      await this.pause();
    }
  }

  /** @param {string} filename @returns {Promise<void>} */
  static async waitPath(filename) {
    const deadline=Date.now()+this.STAGE_TIMEOUT;
    while(!fs.existsSync(filename)) {
      assert.ok(Date.now()<deadline,'Native file/socket did not appear');
      await this.pause();
    }
  }

  /** @returns {Promise<void>} */
  static pause() {
    return new Promise(/** @param {()=>void} resolve @returns {void} */ resolve=>{setTimeout(resolve,this.POLL_INTERVAL);});
  }

  /** @returns {void} */
  static requireNoFailure() {
    if(!fs.existsSync(this.STATUS)) return;
    const status=JSON.parse(fs.readFileSync(this.STATUS,'utf8'));
    assert.notEqual(status.success,false,'Native QA failed: '+JSON.stringify(status));
  }

  /** @param {Promise<number|null>} exit @returns {Promise<number|null>} */
  static async waitExit(exit) {
    /** @type {ReturnType<typeof setTimeout>|undefined} */
    let timer;
    const deadline = new Promise(/** @param {(code:number|null)=>void} _resolve @param {(error:Error)=>void} reject @returns {void} */ (_resolve,reject)=>{
      timer=setTimeout(()=>reject(new Error('Normal Quit did not exit the monitor.')),this.EXIT_TIMEOUT);
    });
    try { return await Promise.race([exit,deadline]); }
    finally { clearTimeout(timer); }
  }
}

await NativeActiveTaskLiveTest.run();
