import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { userInfo } from 'node:os';

class NativeApplicationLifetimeTest {
  /** @type {number} */
  static TIMEOUT = 45000;
  /** @type {number} */
  static POLL_INTERVAL = 100;
  /** @type {string} */
  static ROOT = QaPathUtils.external('OBSERVATORY_QA_ROOT');
  /** @type {string} */
  static BUNDLE = QaPathUtils.external('OBSERVATORY_APP_BUNDLE');
  /** @type {string} */
  static MODE = process.argv[3] ?? 'home-help-home';
  /** @type {string} */
  static LABEL = process.argv[4] ?? 'queue24-lifetime-home';
  /** @type {string} */
  static SOCKET = '/tmp/agent-observatory-' + userInfo().uid + '/hooks-native.sock';
  /** @type {string} */
  static REPORT = path.join(this.ROOT, 'evidence/native-application-lifetime-status.json');
  /** @type {string} */
  static EVENT = QaPathUtils.external('OBSERVATORY_LIFETIME_EVENT_FILE');

  /** @returns {Promise<void>} */
  static async run() {
    assert.ok(['home-help-home', 'help-first'].includes(this.MODE));
    assert.ok(/^[a-z0-9-]+$/.test(this.LABEL));
    const endpointWasPresent = fs.existsSync(this.SOCKET);
    fs.rmSync(this.REPORT, {force: true});
    const log = fs.openSync(path.join(this.ROOT, 'evidence/' + this.LABEL + '-app.txt'), 'w');
    const started = new Date().toISOString();
    const child = spawn(path.join(this.BUNDLE, 'Contents/MacOS/AgentObservatory'), [], {
      env: {...process.env, OBSERVATORY_QA: '', OBSERVATORY_LIFETIME_QA: '1', OBSERVATORY_ROOT: this.ROOT,
        OBSERVATORY_LIFETIME_MODE: this.MODE, OBSERVATORY_LIFETIME_SOCKET: this.SOCKET,
        OBSERVATORY_LIFETIME_EVENT_FILE: this.EVENT},
      stdio: ['ignore', log, log],
    });
    fs.closeSync(log);
    const ended = new Promise(/** @param {(exit: number | null) => void} resolve @param {(error: Error) => void} reject @returns {void} */ (resolve, reject) => {
      child.once('error', reject);
      child.once('exit', resolve);
    });
    await this.waitForReport(child.pid);
    const report = JSON.parse(fs.readFileSync(this.REPORT, 'utf8'));
    fs.copyFileSync(this.REPORT, path.join(this.ROOT, 'evidence/' + this.LABEL + '-status.json'));
    assert.equal(report.success, true, 'Native lifetime QA failed at ' + report.stage);
    const exit = await Promise.race([ended, this.exitTimeout()]);
    assert.equal(exit, 0, 'Normal own-app menu Quit did not exit cleanly.');
    assert.equal(fs.existsSync(this.SOCKET), false, 'Private event socket survived normal Quit.');
    const receipt = {verified_at: new Date().toISOString(), started_at: started, success: true,
      bundle: this.BUNDLE, pid: child.pid, mode: this.MODE, checks: report.checks,
      normal_quit_exit: exit, socket_removed: true, endpoint_was_present_at_start: endpointWasPresent,
      constructor_count: 'not observed',
      scope: 'Actual declared AppKit menus and native SDK; only verified own coordinator report. Capabilities stayed in memory.'};
    fs.writeFileSync(path.join(this.ROOT, 'evidence/' + this.LABEL + '-result.json'), JSON.stringify(receipt, null, 2) + '\n');
    console.log('PASS: ' + this.MODE + ', retained samples/history/socket, stale reader/transfer refusal, Help-time receipt, normal Quit.');
  }

  /** @param {number | undefined} pid @returns {Promise<void>} */
  static async waitForReport(pid) {
    assert.ok(pid && pid > 1, 'Owned native app did not start.');
    const deadline = Date.now() + this.TIMEOUT;
    while (!fs.existsSync(this.REPORT)) {
      assert.ok(Date.now() < deadline, 'Native lifetime helper did not finish within its bound.');
      await this.pause();
    }
  }

  /** @returns {Promise<void>} */
  static pause() {
    return new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, this.POLL_INTERVAL); });
  }

  /** @returns {Promise<never>} */
  static exitTimeout() {
    return new Promise(/** @param {(value: never) => void} _resolve @param {(error: Error) => void} reject @returns {void} */ (_resolve, reject) => {
      const timer = setTimeout(() => { reject(new Error('Normal native Quit did not finish.')); }, this.TIMEOUT);
      timer.unref();
    });
  }
}

await NativeApplicationLifetimeTest.run();
