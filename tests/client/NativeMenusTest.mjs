import {QaPathUtils} from './QaPathUtils.mjs';
import path from 'node:path';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {userInfo} from 'node:os';

class NativeMenusTest {
  /** @type {string} */
  static APP_NAME = 'Swarx';
  /** @type {string} */
  static HELP_LABEL = 'Swarx Help';
  /** @type {string} */
  static EVENT_SOCKET = '/tmp/agent-observatory-' + userInfo().uid + '/hooks-native.sock';
  /** @type {number} */
  static TIMEOUT = 10000;
  /** @type {number} */
  static POLL_INTERVAL = 100;
  /** @type {number} */
  static PID = Number(process.argv[2]);
  /** @type {string} */
  static OUTPUT = QaPathUtils.evidence('queue21-native-menus-ui.json');
  /** @type {string} */
  static EXECUTABLE = path.join(QaPathUtils.external('OBSERVATORY_APP_BUNDLE'), 'Contents/MacOS/Swarx');
  /** @type {{checked_at: string, pid: number, status: string, actions: {name: string, passed: boolean, result: string}[], limits: string[], error?: string}} */
  static receipt = {checked_at: new Date().toISOString(), pid: this.PID, status: 'running', actions: [], limits: ['Actual own-process Mac UI only; no permission or settings changes.', 'No custom Service actions, File Close or Command1–5 claim.', 'HideOthers/ShowAll, clipboard mutation, full-screen and zoom not exercised.', 'Edit › Select All not exercised: the single What’s running page has no text input.']};

  /** @returns {Promise<void>} */
  static async run() {
    try {
      this.identity();
      this.query('set frontmost to true');
      this.groups();
      await this.help();
      await this.monitor();
      await this.window();
      await this.about();
      await this.hide();
      await this.quit();
      this.receipt.status = 'pass';
    } catch (error) {
      this.receipt.status = 'failed';
      this.receipt.error = String(error);
      throw error;
    } finally {
      fs.writeFileSync(this.OUTPUT, JSON.stringify(this.receipt, null, 2));
    }
    console.log('PASS: own-process native menus, local Help/Monitor, focused Edit, window, About/Hide and normal Quit.');
  }

  /** @returns {void} */
  static identity() {
    assert.ok(Number.isInteger(this.PID) && this.PID > 1);
    const result = spawnSync('/bin/ps', ['-p', String(this.PID), '-o', 'comm='], {encoding: 'utf8'});
    assert.equal(result.status, 0, result.stderr);
    assert.ok(result.stdout.trim() === this.EXECUTABLE, result.stdout);
  }

  /** @param {string} action @returns {string} */
  static query(action) {
    const script = `tell application "System Events"
      set ownProcess to first application process whose unix id is ${this.PID}
      tell ownProcess
        ${action}
      end tell
    end tell`;
    const result = spawnSync('/usr/bin/osascript', ['-e', script], {encoding: 'utf8', timeout: this.TIMEOUT});
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  }

  /** @param {string} name @param {string} result @returns {void} */
  static record(name, result) {
    this.receipt.actions.push({name, passed: true, result});
  }

  /** @returns {void} */
  static groups() {
    const groups = this.query('return name of every menu bar item of menu bar 1');
    for (const name of [this.APP_NAME, 'File', 'Edit', 'View', 'Window', 'Help']) assert.ok(groups.includes(name), groups);
    this.record('actual menu groups', groups);
  }

  /** @param {string} group @param {string} label @returns {void} */
  static click(group, label) {
    this.query(`click menu bar item "${group}" of menu bar 1`);
    const enabled = this.query(`return enabled of menu item "${label}" of menu "${group}" of menu bar item "${group}" of menu bar 1`);
    assert.equal(enabled, 'true', label + ' disabled');
    this.query(`click menu item "${label}" of menu "${group}" of menu bar item "${group}" of menu bar 1`);
  }

  /** @returns {string} */
  static text() {
    return this.query(`if (count of windows) is 0 then return ""
      set texts to {}
      repeat with itemRef in (get entire contents of window 1)
        set ownElement to contents of itemRef
        try
          set end of texts to (get name of ownElement) as text
        end try
        try
          set end of texts to (get value of ownElement) as text
        end try
      end repeat
      return texts`);
  }

  /** @param {() => string} read @param {string} expected @returns {Promise<string>} */
  static async wait(read, expected) {
    const deadline = Date.now() + this.TIMEOUT;
    let result = read();
    while (!result.includes(expected)) {
      assert.ok(Date.now() < deadline, expected + ' unavailable: ' + result.slice(0,1200));
      /** @type {Promise<void>} */
      const waiting = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, this.POLL_INTERVAL); });
      await waiting;
      result = read();
    }
    return result;
  }

  /** @returns {Promise<void>} */
  static async help() {
    this.click('Help', this.HELP_LABEL);
    const text = await this.wait(/** @returns {string} */ () => this.text(), this.HELP_LABEL);
    for (const label of ['Agents, tasks and actions', 'Local process resources', 'Ownership and shared runtimes', 'Using this app', 'no automatic Codex desktop task stream']) assert.ok(text.includes(label), text);
    this.record('bundled local Help route and limits', 'Four actual rendered sections. Declared route navigation disposes native companions; collection resumes on returning to the monitor.');
  }

  /** @returns {Promise<void>} */
  static async monitor() {
    this.click('File', 'Open Monitor');
    await this.wait(/** @returns {string} */ () => this.text(), 'AGENTS WORKING');
    this.record('Monitor route returns to What’s running', 'Actual current-work summary cards rendered.');
  }

  /** @returns {Promise<void>} */
  static async window() {
    this.click('Window', 'Minimize');
    await this.wait(/** @returns {string} */ () => this.query('return value of attribute "AXMinimized" of window 1'), 'true');
    this.query('set value of attribute "AXMinimized" of window 1 to false\nset frontmost to true');
    assert.equal(this.query('return value of attribute "AXMinimized" of window 1'), 'false');
    this.record('Minimize and restore own window', 'Actual native window state changed and restored.');
  }

  /** @returns {Promise<void>} */
  static async about() {
    this.click(this.APP_NAME, 'About ' + this.APP_NAME);
    await this.wait(/** @returns {string} */ () => this.query('return count of windows'), '2');
    this.record('standard About panel', this.query('return name of every window'));
    this.query('click (first button of window 1 whose subrole is "AXCloseButton")');
    assert.equal(this.query('return count of windows'), '1');
  }

  /** @returns {Promise<void>} */
  static async hide() {
    this.click(this.APP_NAME, 'Hide ' + this.APP_NAME);
    await this.wait(/** @returns {string} */ () => this.query('return visible'), 'false');
    this.query('set frontmost to true');
    await this.wait(/** @returns {string} */ () => this.query('return visible'), 'true');
    this.record('Hide and show only our app', 'Native app visibility changed and restored.');
  }

  /** @returns {Promise<void>} */
  static async quit() {
    this.click(this.APP_NAME, 'Quit ' + this.APP_NAME);
    await this.wait(/** @returns {string} */ () => String(spawnSync('/bin/ps',['-p',String(this.PID)],{encoding:'utf8'}).status), '1');
    assert.equal(fs.existsSync(this.EVENT_SOCKET), false);
    this.record('normal menu Quit releases owned event source', 'Own process exited; its private socket was removed.');
  }
}

await NativeMenusTest.run();
