import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn, spawnSync} from 'node:child_process';

/** @typedef {import('../app/client/models/CommandOwner.ts').CommandOwner} CommandOwner */
/** @typedef {{pid: number, started: string, user: number, executable: string}} ProcessIdentity */
/** @typedef {{id: string, generation: number}} BindingIdentity */
/** @typedef {{accepted: boolean, revision: number | null, error: string | null, command?: {accepted: boolean, error: string, binding: BindingIdentity | null}}} LogicalReceipt */
/** @typedef {{type?: string, kind?: string, version?: number, owner?: CommandOwner, identity?: ProcessIdentity, binding_id?: string | null, agent?: string, session_id?: string, session_name?: string, name?: string, family?: string, status?: string, task?: string, task_name?: string, execution?: string, exit_code?: number, duration_seconds?: number}} SentEvent */
/** @typedef {{type: string | undefined, accepted: boolean, revision: number | null, binding?: string, generation?: number}} ObservedReceipt */
/** @typedef {{code: number | null, signal: NodeJS.Signals | null}} ChildExit */

class NativeCommandLiveTest {
  /** @type {number} */
  static SUCCESS_STATUS = 0;
  /** @type {number} */
  static HELPER_TIMEOUT = 5000;
  /** @type {number} */
  static READY_TIMEOUT = 10000;
  /** @type {number} */
  static READY_POLL_INTERVAL = 250;
  /** @type {number} */
  static RENEWAL_INTERVAL = 20000;
  /** @type {number} */
  static WIRE_VERSION = 1;
  /** @type {number} */
  static JSON_INDENTATION = 2;
  /** @type {number} */
  static NANOSECONDS_PER_SECOND = 1e9;
  /** @type {BufferEncoding} */
  static FILE_ENCODING = 'utf8';
  /** @type {string} */
  static COORDINATOR = QaPathUtils.required('OBSERVATORY_QA_AGENT_ID');
  /** @type {string} */
  static EVIDENCE_PATH = 'evidence/';
  /** @type {string} */
  static ROOT = QaPathUtils.external('OBSERVATORY_QA_ROOT');
  /** @type {string} */
  static HELPER = path.join(QaPathUtils.external('OBSERVATORY_APP_BUNDLE'), 'Contents/MacOS/observatory-event');
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
  static EVIDENCE_LABEL = process.env.OBSERVATORY_QA_BUILD ?? 'queue14';
  /** @type {ObservedReceipt[]} */
  static receipts = [];

  /** @param {SentEvent} event @returns {LogicalReceipt} */
  static event(event) {
    const result = spawnSync(this.HELPER, [], {input: JSON.stringify(event), encoding: this.FILE_ENCODING, timeout: this.HELPER_TIMEOUT});
    assert.equal(result.status, this.SUCCESS_STATUS, result.stderr + result.stdout);
    /** @type {LogicalReceipt} */
    const receipt = JSON.parse(result.stdout);
    assert.equal(receipt.accepted, true, receipt.error ?? undefined);
    this.receipts.push({type: event.type ?? event.kind, accepted: receipt.accepted, revision: receipt.revision, binding: receipt.command?.binding?.id, generation: receipt.command?.binding?.generation});
    return receipt;
  }

  /** @returns {Promise<void>} */
  static async roster() {
    await this.sourceReady([this.COORDINATOR, 'running', this.TASK, this.TASK_NAME]);
  }

  /** @param {[string, string, string, string]} registration @returns {Promise<void>} */
  static async sourceReady([agent, status, task, taskName]) {
    const event = {type: 'logical.coordination.upsert', agent, session_id: this.SESSION, session_name: this.SESSION_LABEL, name: agent, family: this.FAMILY, status, task, task_name: taskName};
    const deadline = Date.now() + this.READY_TIMEOUT;
    for (;;) {
      const result = spawnSync(this.HELPER, [], {input: JSON.stringify(event), encoding: this.FILE_ENCODING, timeout: this.HELPER_TIMEOUT});
      if (result.status === this.SUCCESS_STATUS) {
        /** @type {LogicalReceipt} */
        const receipt = JSON.parse(result.stdout);
        assert.equal(receipt.accepted, true, receipt.error ?? undefined);
        this.receipts.push({type: event.type, accepted: true, revision: receipt.revision});
        return;
      }
      assert.ok(result.stderr.includes('private event source is unavailable'), result.stderr + result.stdout);
      assert.ok(Date.now() < deadline, 'Native event endpoint did not become ready.');
      /** @type {Promise<void>} */
      const waiting = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, this.READY_POLL_INTERVAL); });
      await waiting;
    }
  }

  /** @param {number | undefined} pid @returns {ProcessIdentity} */
  static identity(pid) {
    const result = spawnSync(this.HELPER, ['--identity', String(pid)], {encoding: this.FILE_ENCODING, timeout: this.HELPER_TIMEOUT});
    assert.equal(result.status, this.SUCCESS_STATUS, result.stderr);
    /** @type {ProcessIdentity} */
    const identity = JSON.parse(result.stdout);
    assert.equal(identity.pid, pid);
    return identity;
  }

  /** @param {string} kind @param {CommandOwner} owner @param {ProcessIdentity} identity @param {string | null} bindingId @returns {LogicalReceipt} */
  static binding(kind, owner, identity, bindingId) {
    return this.event({version: this.WIRE_VERSION, kind, owner, identity, binding_id: bindingId});
  }

  /** @param {number} parent @returns {Promise<ProcessIdentity[]>} */
  static async descendantIdentities(parent) {
    const deadline = Date.now() + this.HELPER_TIMEOUT;
    for (;;) {
      const sampled = spawnSync('/usr/bin/pgrep', ['-P', String(parent)], {encoding: this.FILE_ENCODING, timeout: this.HELPER_TIMEOUT});
      if (sampled.status === this.SUCCESS_STATUS) {
        const identifiers = sampled.stdout.trim().split(/\s+/).map(/** @param {string} value @returns {number} */ value => Number(value));
        return identifiers.map(/** @param {number} pid @returns {ProcessIdentity} */ pid => this.identity(pid));
      }
      assert.ok(Date.now() < deadline, 'Actual controlled shell child did not appear.');
      /** @type {Promise<void>} */
      const waiting = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, this.READY_POLL_INTERVAL); });
      await waiting;
    }
  }

  /** @returns {Promise<void>} */
  static async run() {
    await this.roster();
    const execution = 'controlled-child-' + Date.now();
    const owner = {agent_id: this.COORDINATOR, session_id: this.SESSION, task_id: this.TASK, execution_id: execution};
    const started = process.hrtime.bigint();
    const child = spawn('/bin/sh', ['-c', '/bin/sleep 32 & wait'], {stdio: 'ignore'});
    /** @type {Promise<ChildExit>} */
    const exited = new Promise(/** @param {(value: ChildExit) => void} resolve @returns {void} */ resolve => {
      child.once('exit', /** @param {number | null} code @param {NodeJS.Signals | null} signal @returns {void} */ (code, signal) => resolve({code, signal}));
    });
    /** @type {Promise<void>} */
    const spawned = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { child.once('spawn', resolve); });
    await spawned;
    this.event({type: 'logical.coordination.execution.start', agent: this.COORDINATOR, session_id: this.SESSION, task: this.TASK, execution, name: this.ACTION});
    const identity = this.identity(child.pid);
    const childIdentities = await this.descendantIdentities(identity.pid);
    const receipt = this.binding('logical.command.bind', owner, identity, null);
    const binding = receipt.command?.binding;
    assert.ok(binding);
    fs.writeFileSync(path.join(this.ROOT, this.EVIDENCE_PATH + this.EVIDENCE_LABEL + '-command-live-start.json'), JSON.stringify({pid: child.pid, started: identity.started, session: this.SESSION, task: this.TASK, execution, bindingId: binding.id, generation: binding.generation, childIdentities, scope: 'explicit caller-controlled shell and actual sleep child; individual process metrics, not agent/model totals', receipts: this.receipts}, null, this.JSON_INDENTATION));
    const renewal = setTimeout(/** @returns {void} */ () => {
      this.binding('logical.command.renew', owner, this.identity(child.pid), binding.id);
    }, this.RENEWAL_INTERVAL);
    const result = await exited;
    clearTimeout(renewal);
    assert.equal(result.code, this.SUCCESS_STATUS);
    const duration = Number(process.hrtime.bigint() - started) / this.NANOSECONDS_PER_SECOND;
    this.event({type: 'logical.coordination.execution.finish', agent: this.COORDINATOR, session_id: this.SESSION, task: this.TASK, execution, status: 'completed', exit_code: this.SUCCESS_STATUS, duration_seconds: duration});
    fs.writeFileSync(path.join(this.ROOT, this.EVIDENCE_PATH + this.EVIDENCE_LABEL + '-command-live-result.json'), JSON.stringify({success: true, pid: child.pid, started: identity.started, session: this.SESSION, task: this.TASK, execution, bindingId: binding.id, durationSeconds: duration, childIdentities, measuredExit: result.code, scope: 'actual controlled process and opt-in lifecycle; UI must be independently verified', receipts: this.receipts}, null, this.JSON_INDENTATION));
    console.log('PASS: real controlled command identity, bind, renewal and actual exit lifecycle.');
  }
}

await NativeCommandLiveTest.run();
