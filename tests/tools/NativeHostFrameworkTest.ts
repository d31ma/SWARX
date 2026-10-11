import test, { type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { NativeHostCustomizer } from '../../tools/NativeHostCustomizer.ts';
import { NativePackagerFixture } from './NativePackagerFixture.ts';

const GENERATED_MENU = 'menus.install(NSApplication.shared, window, "Swarx", menuNavigation, menuActions)';
const GENERATED_TITLE = 'window.title = "Swarx"';
const ACTIVATION = '        NSApplication.shared.activate(ignoringOtherApps: true)\n';

test('current framework host keeps the generated Swarx title and menu actions when inserting QA', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source = fs.readFileSync(fixture.host(), 'utf8');
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  const customized = customizer.customize(source);
  assert.ok(customized.includes(GENERATED_MENU));
  assert.ok(customized.includes(GENERATED_TITLE));
  assert.ok(customized.includes('window.contentMinSize = NSSize(width: 900, height: 640)'));
  assert.ok(customized.includes('Telemetry.shared.record("controller.active")'));
  assert.ok(customized.includes('NativePreviewVerifier.begin(view: webView, window: window)'));
  assert.ok(customized.includes('NativeApplicationLifetimeVerifier.begin(view: webView, window: window)'));
  assert.equal(customizer.customize(customized), customized);
});

test('missing framework activation anchor fails before rewriting source or invoking tools', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const file = fixture.host();
  const source = fs.readFileSync(file, 'utf8').replace(ACTIVATION, '');
  fs.writeFileSync(file, source);
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  assert.throws((): void => customizer.build(), /generation anchor/);
  assert.equal(fs.readFileSync(file, 'utf8'), source);
  assert.equal(fixture.commands.length, 0);
});
