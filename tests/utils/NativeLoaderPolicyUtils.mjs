import fs from 'node:fs';
import path from 'node:path';

/** @typedef {{status: number | null, stdout: string, error: string | null}} LoaderToolResult */
/** @typedef {{references: string[], errors: string[]}} LoaderReferences */
/** @typedef {{reference: string, passed: boolean, kind: string, target: string | null}} LoaderPathCheck */
/** @typedef {{passed: boolean, dependencies: LoaderPathCheck[], rpaths: LoaderPathCheck[], errors: string[]}} LoaderPolicy */
/** @typedef {{binary: string, macro: string | undefined}} LoaderExpansion */

/** Static packaging policy for the two bundled executables, not a dyld simulator. */
export class NativeLoaderPolicyUtils {
  /** @type {readonly string[]} */
  static #SYSTEM_ROOTS = ['/usr/lib', '/System/Library'];
  /** @type {readonly string[]} */
  static #LOCAL_MACROS = ['@loader_path', '@executable_path'];

  /** @param {string} bundle @param {string} binary @param {LoaderToolResult} libraries @param {LoaderToolResult} commands @returns {LoaderPolicy} */
  static inspect(bundle, binary, libraries, commands) {
    const parsedLibraries = this.#libraries(binary, libraries);
    const parsedPaths = this.#paths(binary, commands);
    const rpaths = parsedPaths.references.map(/** @param {string} reference @returns {LoaderPathCheck} */ reference => this.#rpath(bundle, binary, reference));
    const dependencies = parsedLibraries.references.map(/** @param {string} reference @returns {LoaderPathCheck} */ reference => this.#dependency(bundle, binary, reference, rpaths));
    const errors = [...parsedLibraries.errors, ...parsedPaths.errors];
    const passed = [errors.length === 0, ...rpaths.map(this.#passed), ...dependencies.map(this.#passed)].every(Boolean);
    return {passed, dependencies, rpaths, errors};
  }

  /** @param {LoaderPathCheck} entry @returns {boolean} */
  static #passed(entry) { return entry.passed; }

  /** @param {string} binary @param {LoaderToolResult} result @returns {LoaderReferences} */
  static #libraries(binary, result) {
    const metadata = this.#metadata(binary, result);
    const lines = result.stdout.trim().split('\n').slice(1).filter(/** @param {string} line @returns {boolean} */ line => line.trim().length > 0);
    const matches = lines.map(/** @param {string} line @returns {RegExpMatchArray | null} */ line => line.trim().match(/^(.+) \(compatibility version [^)]*\)$/));
    const valid = [metadata.length === 0, lines.length > 0, matches.every(/** @param {RegExpMatchArray | null} match @returns {boolean} */ match => match !== null)].every(Boolean);
    return this.#libraryResult(valid, matches, metadata);
  }

  /** @param {boolean} valid @param {(RegExpMatchArray | null)[]} matches @param {string[]} errors @returns {LoaderReferences} */
  static #libraryResult(valid, matches, errors) {
    if (!valid) return {references:[], errors:[...errors, 'Dependency metadata is empty or malformed.']};
    return {references:matches.map(/** @param {RegExpMatchArray | null} match @returns {string} */ match => /** @type {RegExpMatchArray} */ (match)[1]), errors:[]};
  }

  /** @param {string} binary @param {LoaderToolResult} result @returns {LoaderReferences} */
  static #paths(binary, result) {
    const errors = this.#metadata(binary, result);
    const blocks = result.stdout.split(/^Load command \d+\s*$/m).slice(1);
    const parsed = blocks.map(/** @param {string} block @returns {LoaderReferences} */ block => this.#pathCommand(block));
    return {references:parsed.flatMap(/** @param {LoaderReferences} item @returns {string[]} */ item => item.references),
      errors:[...errors, ...this.#missingCommands(blocks), ...parsed.flatMap(/** @param {LoaderReferences} item @returns {string[]} */ item => item.errors)]};
  }

  /** @param {string[]} blocks @returns {string[]} */
  static #missingCommands(blocks) {
    if (blocks.length === 0) return ['Load-command metadata is empty or malformed.'];
    return [];
  }

  /** @param {string} binary @param {LoaderToolResult} result @returns {string[]} */
  static #metadata(binary, result) {
    const valid = [result.status === 0, result.error === null, result.stdout.split('\n')[0] === binary+':', !result.stdout.includes('(architecture ')].every(Boolean);
    return this.#metadataErrors(valid);
  }

  /** @param {boolean} valid @returns {string[]} */
  static #metadataErrors(valid) {
    if (!valid) return ['Cannot verify single-image otool metadata for the requested executable.'];
    return [];
  }

  /** @param {string} block @returns {LoaderReferences} */
  static #pathCommand(block) {
    if (!/^\s*cmd\s+LC_\w+\s*$/m.test(block)) return {references:[], errors:['Load command has no readable command type.']};
    return this.#rpathCommand(block);
  }

  /** @param {string} block @returns {LoaderReferences} */
  static #rpathCommand(block) {
    if (!/^\s*cmd\s+LC_RPATH\s*$/m.test(block)) return {references:[], errors:[]};
    return this.#pathValue([...block.matchAll(/^\s*path (.+) \(offset \d+\)\s*$/gm)]);
  }

  /** @param {RegExpMatchArray[]} paths @returns {LoaderReferences} */
  static #pathValue(paths) {
    if (paths.length !== 1) return {references:[], errors:['LC_RPATH does not declare exactly one readable path.']};
    return {references:[paths[0][1]], errors:[]};
  }

  /** @param {string} bundle @param {string} binary @param {string} reference @returns {LoaderPathCheck} */
  static #rpath(bundle, binary, reference) {
    if (path.isAbsolute(reference)) return this.#system(reference, path.normalize(reference));
    return this.#localReference(bundle, binary, reference, true);
  }

  /** @param {string} bundle @param {string} binary @param {string} reference @param {LoaderPathCheck[]} rpaths @returns {LoaderPathCheck} */
  static #dependency(bundle, binary, reference, rpaths) {
    if (path.isAbsolute(reference)) return this.#system(reference, path.normalize(reference));
    return this.#relativeDependency(bundle, binary, reference, rpaths);
  }

  /** @param {string} bundle @param {string} binary @param {string} reference @param {LoaderPathCheck[]} rpaths @returns {LoaderPathCheck} */
  static #relativeDependency(bundle, binary, reference, rpaths) {
    if (reference.startsWith('@rpath/')) return this.#searched(bundle, reference, rpaths);
    return this.#localReference(bundle, binary, reference, false);
  }

  /** @param {string} reference @param {string} target @returns {LoaderPathCheck} */
  static #system(reference, target) {
    const system = NativeLoaderPolicyUtils.#SYSTEM_ROOTS.some(/** @param {string} root @returns {boolean} */ root => this.#inside(root, target));
    const passed = [system, !/[\u0000-\u001f]/.test(reference)].every(Boolean);
    return {reference, passed, kind:passed ? 'system-declared' : 'absolute-external-path', target};
  }

  /** @param {string} bundle @param {string} binary @param {string} reference @param {boolean} directory @returns {LoaderPathCheck} */
  static #localReference(bundle, binary, reference, directory) {
    const macro = NativeLoaderPolicyUtils.#LOCAL_MACROS.find(/** @param {string} candidate @returns {boolean} */ candidate => this.#macro(reference, candidate));
    return this.#expanded(bundle, reference, {binary, macro}, directory);
  }

  /** @param {string} reference @param {string} macro @returns {boolean} */
  static #macro(reference, macro) { return [reference === macro, reference.startsWith(macro+'/')].some(Boolean); }

  /** @param {string} bundle @param {string} reference @param {LoaderExpansion} expansion @param {boolean} directory @returns {LoaderPathCheck} */
  static #expanded(bundle, reference, expansion, directory) {
    if (expansion.macro === undefined) return {reference, passed:false, kind:'unsupported-relative-path', target:null};
    const suffix = reference.slice(expansion.macro.length).replace(/^\//, '');
    return this.#local(bundle, reference, path.resolve(path.dirname(expansion.binary), suffix), directory);
  }

  /** @param {string} bundle @param {string} reference @param {LoaderPathCheck[]} rpaths @returns {LoaderPathCheck} */
  static #searched(bundle, reference, rpaths) {
    const candidates = rpaths.filter(this.#passed).map(/** @param {LoaderPathCheck} root @returns {LoaderPathCheck} */ root => this.#candidate(bundle, reference, root));
    return candidates.find(this.#passed) ?? {reference, passed:false, kind:'unresolved-rpath', target:null};
  }

  /** @param {string} bundle @param {string} reference @param {LoaderPathCheck} root @returns {LoaderPathCheck} */
  static #candidate(bundle, reference, root) {
    const target = path.resolve(/** @type {string} */ (root.target), reference.slice('@rpath/'.length));
    if (root.kind === 'system-declared') return this.#system(reference, target);
    return this.#local(bundle, reference, target, false);
  }

  /** @param {string} bundle @param {string} reference @param {string} target @param {boolean} directory @returns {LoaderPathCheck} */
  static #local(bundle, reference, target, directory) {
    if (!this.#inside(bundle, target)) return {reference, passed:false, kind:'bundle-path-escape', target};
    return this.#physical(bundle, reference, target, directory);
  }

  /** @param {string} bundle @param {string} reference @param {string} target @param {boolean} directory @returns {LoaderPathCheck} */
  static #physical(bundle, reference, target, directory) {
    try {
      const physical = fs.realpathSync(target);
      const information = fs.statSync(physical);
      const type = directory ? information.isDirectory() : [information.isFile(), information.size > 0].every(Boolean);
      const passed = [this.#inside(fs.realpathSync(bundle), physical), type].every(Boolean);
      return {reference, passed, kind:passed ? 'bundle-relative-present' : 'invalid-physical-bundle-target', target};
    } catch {
      return {reference, passed:false, kind:'missing-bundle-target', target};
    }
  }

  /** @param {string} root @param {string} target @returns {boolean} */
  static #inside(root, target) {
    const relative = path.relative(root, target);
    return [relative !== '..', !relative.startsWith('..'+path.sep), !path.isAbsolute(relative)].every(Boolean);
  }
}
