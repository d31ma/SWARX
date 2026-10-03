import {QaPathUtils} from './QaPathUtils.mjs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export class OverviewFixtureClient {
  /** @type {URL} */
  static BINARY = QaPathUtils.file('OBSERVATORY_OVERVIEW_FIXTURE');
  /** @type {number} */
  static MAXIMUM_BYTES = 2_097_152;
  /** @type {number} */
  static TIMEOUT_MILLISECONDS = 5000;

  /** @param {unknown} request @returns {import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel} */
  static project(request) {
    const output = OverviewFixtureClient.call(request);
    return JSON.parse(output);
  }

  /** @param {unknown} request @returns {string} */
  static call(request) {
    return execFileSync(fileURLToPath(OverviewFixtureClient.BINARY), [], {
      input: JSON.stringify(request),
      encoding: 'utf8',
      maxBuffer: OverviewFixtureClient.MAXIMUM_BYTES,
      timeout: OverviewFixtureClient.TIMEOUT_MILLISECONDS
    });
  }
}
