import test, { type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { NativeHostCustomizer } from '../../tools/NativeHostCustomizer.ts';
import { NativePackagerFixture } from './NativePackagerFixture.ts';

const LEGACY_MENU = 'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation)';
const CURRENT_MENU = 'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation, menuActions)';
const BRANDED_MENU = 'menus.install(NSApplication.shared, window, "Swarx", menuNavigation, menuActions)';

test('current framework host retains application menu actions when branding and inserting QA', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source = fs.readFileSync(fixture.host(), 'utf8').replace(LEGACY_MENU, CURRENT_MENU);
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  const branded = customizer.customize(source);
  assert.ok(branded.includes(BRANDED_MENU));
  assert.ok(branded.includes('window.title = "Swarx Agent Observatory"'));
  assert.ok(branded.includes('window.contentMinSize = NSSize(width: 900, height: 640)'));
  assert.ok(branded.includes('Telemetry.shared.record("controller.active")'));
  assert.ok(branded.includes('NativePreviewVerifier.begin(view: webView, window: window)'));
  assert.ok(branded.includes('NativeApplicationLifetimeVerifier.begin(view: webView, window: window)'));
  assert.equal(customizer.customize(branded), branded);
});

test('unrecognized framework menu arguments fail before rewriting source or invoking tools', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const file = fixture.host();
  const source = fs.readFileSync(file, 'utf8').replace(LEGACY_MENU,
    'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation, unknownActions)');
  fs.writeFileSync(file, source);
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  assert.throws((): void => customizer.build(), /branding anchor/);
  assert.equal(fs.readFileSync(file, 'utf8'), source);
  assert.equal(fixture.commands.length, 0);
});
