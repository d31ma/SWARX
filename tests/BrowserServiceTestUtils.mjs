import {readFileSync} from 'node:fs';
import vm from 'node:vm';

export class BrowserServiceTestUtils {
  /** @param {vm.Context} context @param {string} name @param {string} folder @returns {void} */
  static load(context, name, folder = 'services/browser') {
    const url = new URL('../app/client/' + folder + '/' + name + '.js', import.meta.url);
    const source = BrowserServiceTestUtils.source(url);
    vm.runInContext(source + '\nglobalThis.' + name + ' = ' + name + ';', context, {filename: url.href});
  }

  /** @param {URL} url @returns {string} */
  static source(url) {
    return readFileSync(url, 'utf8')
      .replace(/^import .*;\n/gm, '')
      .replace(/^\s*@(?:Service|Needs|OnStart|OnStop)\s*$/gm, '')
      .replaceAll('import.meta.url', JSON.stringify(url.href))
      .replace('export default ', '')
      .replace('export class ', 'class ');
  }
}
