import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {ToolPathUtils} from '../../tools/utils/ToolPathUtils.ts';

export class QaPathUtils {
  /** @param {string} name @returns {string} */
  static required(name) { return ToolPathUtils.required(name); }

  /** @param {string} name @returns {string} */
  static external(name) { return ToolPathUtils.external(fileURLToPath(new URL('../../', import.meta.url)), name); }

  /** @param {string} name @returns {URL} */
  static file(name) { return pathToFileURL(this.external(name)); }

  /** @param {string} name @returns {URL} */
  static directory(name) { return pathToFileURL(this.external(name) + path.sep); }

  /** @param {string} name @returns {URL} */
  static evidence(name) { return new URL('evidence/' + name, this.directory('OBSERVATORY_QA_ROOT')); }
}
