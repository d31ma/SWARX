import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// Sessions are grouped by exact provider ID, so the card and detail panel must show it beside the title.
class SessionIdentityTemplateTest {
  /** @param {string} component @returns {string} */
  static template(component) {
    return readFileSync(new URL('../../app/client/components/observatory/' + component + '/tac.html', import.meta.url), 'utf8');
  }
}

test('session cards show the exact provider session ID or say it is not reported', /** @returns {void} */ () => {
  assert.match(SessionIdentityTemplateTest.template('running'), /<p class="session-id"[^>]*>Session ID · \{session\.id \|\| 'not reported'\}<\/p>/);
});

test('the session panel shows the exact provider session ID or says it is not reported', /** @returns {void} */ () => {
  assert.match(SessionIdentityTemplateTest.template('session'), /<p class="session-id"[^>]*>Session ID · \{observatory\.model\.sessionDetail\.id \|\| 'not reported'\}<\/p>/);
});

test('a Remote Control session card shows its exact Remote Control ID and that the project thread is not reported locally', /** @returns {void} */ () => {
  assert.match(SessionIdentityTemplateTest.template('running'), /<logic :if="session\.remoteControlId"><p class="session-id"[^>]*>Remote Control ID · \{session\.remoteControlId\} · project thread not reported locally<\/p><\/logic>/);
});

test('the session panel shows the exact Remote Control ID only when one is reported', /** @returns {void} */ () => {
  assert.match(SessionIdentityTemplateTest.template('session'), /<logic :if="observatory\.model\.sessionDetail\.remoteControlId"><p class="session-id">Remote Control ID · \{observatory\.model\.sessionDetail\.remoteControlId\} · project thread not reported locally<\/p><\/logic>/);
});
