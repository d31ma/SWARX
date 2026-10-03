import fs from 'node:fs';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {spawnSync} from 'node:child_process';
import path from 'node:path';
import {NativeHostCustomizer} from '../../tools/NativeHostCustomizer.ts';
import {IconPackager} from '../../tools/IconPackager.ts';
import {NativePackagerFixture} from '../tools/NativePackagerFixture.ts';

class BrandingTest {
  /** @type {URL} */
  static ROOT = new URL('../../', import.meta.url);
  /** @type {string} */
  static ORIGINAL_WINDOW = 'window.title = "Agent Observatory"';
  /** @type {string} */
  static ORIGINAL_MENU = 'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation)';

  /** @param {NativePackagerFixture} fixture @returns {string} */
  static host(fixture) {
    const file=fixture.host();
    return fs.readFileSync(file,'utf8')
      + '\nlet identity = "ma.del.agent-observatory"\nlet executable = "AgentObservatory"\nlet storage = "agent-observatory"\n';
  }

  /** @param {string} file @returns {string} */
  static source(file) { return fs.readFileSync(new URL(file,this.ROOT),'utf8'); }

  /** @param {string} file @returns {Record<string, string>} */
  static plist(file) {
    const result=spawnSync('/usr/bin/plutil',['-convert','json','-o','-',file],{encoding:'utf8'});
    assert.equal(result.status,0,result.stderr);
    return JSON.parse(result.stdout);
  }
}

test('native customization brands window and system-menu display arguments while retaining identities', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture=new NativePackagerFixture();
  context.after(/** @returns {void} */ () => fixture.close());
  const customizer=new NativeHostCustomizer(fixture.root,fixture.run);
  const branded=customizer.customize(BrandingTest.host(fixture));
  assert.ok(branded.includes('window.title = "Swarx Agent Observatory"'));
  assert.ok(branded.includes('menus.install(NSApplication.shared, window, "Swarx", menuNavigation)'));
  assert.ok(branded.includes('let identity = "ma.del.agent-observatory"'));
  assert.ok(branded.includes('let executable = "AgentObservatory"'));
  assert.ok(branded.includes('let storage = "agent-observatory"'));
  assert.equal(customizer.customize(branded),branded);
});

test('native branding rejects a changed window-title generation contract', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture=new NativePackagerFixture();
  context.after(/** @returns {void} */ () => fixture.close());
  const customizer=new NativeHostCustomizer(fixture.root,fixture.run);
  const host=BrandingTest.host(fixture).replaceAll(BrandingTest.ORIGINAL_WINDOW,'window.title = "Unknown generated title"');
  assert.throws(/** @returns {string} */ () => customizer.customize(host),/branding anchor/);
});

test('native branding rejects a changed application-menu generation contract', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture=new NativePackagerFixture();
  context.after(/** @returns {void} */ () => fixture.close());
  const customizer=new NativeHostCustomizer(fixture.root,fixture.run);
  const host=BrandingTest.host(fixture).replaceAll(BrandingTest.ORIGINAL_MENU,'menus.install(NSApplication.shared, window, "Unknown generated name", menuNavigation)');
  assert.throws(/** @returns {string} */ () => customizer.customize(host),/branding anchor/);
});

test('native bundle display metadata becomes Swarx while executable and application IDs stay unchanged', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture=new NativePackagerFixture();
  context.after(/** @returns {void} */ () => fixture.close());
  fixture.icon();
  const plist=fixture.write('app/dist/macos/AgentObservatory.app/Contents/Info.plist','<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>local.fixture.monitor</string><key>CFBundleExecutable</key><string>AgentObservatory</string><key>CFBundleDisplayName</key><string>Agent Observatory</string><key>CFBundleName</key><string>Agent Observatory</string><key>CFBundleVersion</key><string>17</string></dict></plist>');
  new IconPackager(fixture.root,fixture.run).package();
  const information=BrandingTest.plist(plist);
  assert.equal(information.CFBundleDisplayName,'Swarx');
  assert.equal(information.CFBundleName,'Swarx');
  assert.equal(information.CFBundleIdentifier,'local.fixture.monitor');
  assert.equal(information.CFBundleExecutable,'AgentObservatory');
  assert.equal(information.CFBundleVersion,'17');
  assert.equal(fs.existsSync(path.join(fixture.artifacts,'macos/AgentObservatory.app')),true);
  assert.equal(information.CFBundleIconFile,'Observatory.icns');
});

test('client header and Help show Swarx with the retained logo asset', /** @returns {void} */ () => {
  const panel=BrandingTest.source('app/client/components/observatory/panel/tac.html');
  const help=BrandingTest.source('app/client/components/observatory/help/tac.html');
  assert.match(panel,/<strong>Swarx Agent Observatory<\/strong>/);
  assert.ok(panel.includes('/shared/assets/observatory.svg'));
  assert.ok(help.includes('<h1>Swarx Help</h1>'));
  assert.ok(help.includes('alt="Swarx logo"'));
});

test('supported client metadata and declared Help label use Swarx while packaging name and ID remain stable', /** @returns {void} */ () => {
  /** @type {{name: string, tac:{application:{id:string,menus:{help:{label:string}[]}},metadata?:{defaults:{title:string},routes:Record<string,{title:string}>}}}} */
  const manifest=JSON.parse(BrandingTest.source('app/client/manifest.json'));
  assert.equal(manifest.name,'Agent Observatory');
  assert.equal(manifest.tac.application.id,'ma.del.agent-observatory');
  assert.equal(manifest.tac.application.menus.help[0]?.label,'Swarx Help');
  assert.equal(manifest.tac.metadata?.defaults.title,'Swarx Agent Observatory');
  assert.equal(manifest.tac.metadata?.routes['/help']?.title,'Swarx Help');
});
