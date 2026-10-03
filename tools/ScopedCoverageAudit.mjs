import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

/** @typedef {import('node:inspector').Profiler.FunctionCoverage} CoveredFunction */
/** @typedef {import('node:inspector').Profiler.ScriptCoverage} CoveredScript */
/** @typedef {{result: CoveredScript[]}} CoverageProfile */
/** @typedef {{text:string,start:number,end:number}} SourceToken */
/** @typedef {{scan:()=>number,getTokenText:()=>string,getTokenStart:()=>number,getTokenEnd:()=>number}} SourceScanner */
/** @typedef {{createScanner:(skip:boolean,variant:number,source:string)=>SourceScanner,LanguageVariant:{Standard:number},SyntaxKind:{EndOfFile:number}}} ScannerModule */
/** @typedef {{total_lines:number,covered_lines:number,line_coverage:number,total_tokens:number,covered_tokens:number,token_coverage:number}} CoverageMeasurement */
/** @typedef {CoverageMeasurement & {name:string,start_line:number,end_line:number,calls:number,syntactic_complexity:number,crap:number}} MethodMeasurement */
/** @typedef {{file:string,sha256:string,vm_instances:number,methods:MethodMeasurement[]}} FileMeasurement */
/** @typedef {{line:number,covered:boolean}} MeasuredToken */
/** @typedef {{measured_at:string,node:string,scope:string[],parser:string,coverage:string,complexity:string,line_definition:string,formula:string,limitations:string[],files:FileMeasurement[]}} CoverageReport */

class ScopedCoverageAudit {
  /** @type {number} */
  static ARGUMENT_SKIP = 2;
  /** @type {number} */
  static #ARGUMENT_COUNT = 3;
  /** @type {number} */
  static #EMPTY_COUNT = 0;
  /** @type {number} */
  static #JSON_INDENTATION = 2;
  /** @type {string} */
  static #LINE_BREAK = '\n';
  /** @type {BufferEncoding} */
  static #FILE_ENCODING = 'utf8';
  /** @type {string[]} */
  static #FILES = [
    'app/client/utils/ScriptUtils.js',
    'app/client/services/native/monitoring/SnapshotService.js'
  ];
  /** @type {Set<string>} */
  static #DECISIONS = new Set(['if', 'for', 'while', 'do', 'case', 'catch', '?', '&&', '||', '??']);
  /** @type {Set<string>} */
  static #PUNCTUATION = new Set(['{', '}', '(', ')', ';', ',', '.', '[', ']']);
  /** @type {number} */
  static #EXPONENT = 3;
  /** @type {number} */
  static #MAXIMUM_MILLISECONDS = 10_000;
  /** @type {number} */
  static #MAXIMUM_PROFILES = 8;
  /** @type {number} */
  static #MAXIMUM_PROFILE_BYTES = 5_242_880;
  /** @type {ScannerModule} */
  #parser;
  /** @type {string} */
  #directory;
  /** @type {string} */
  #output;

  /** @param {ScannerModule} parser @param {string} directory @param {string} output */
  constructor(parser, directory, output) {
    this.#parser = parser;
    this.#directory = directory;
    this.#output = output;
  }

  /** @param {string[]} argumentsList @returns {Promise<void>} */
  static async main(argumentsList) {
    const configuration = ScopedCoverageAudit.#configuration(argumentsList);
    const parser = await import(pathToFileURL(resolve(configuration.parser)).href);
    const audit = new ScopedCoverageAudit(parser, configuration.directory, configuration.output);
    audit.run();
  }

  /** @returns {void} */
  run() {
    const profiles = this.#profiles();
    const files = this.#files(profiles);
    const report = this.#report(files);
    const summary = this.#summary(files);
    writeFileSync(this.#output, JSON.stringify(report, null, ScopedCoverageAudit.#JSON_INDENTATION) + ScopedCoverageAudit.#LINE_BREAK);
    process.stdout.write(JSON.stringify(summary) + ScopedCoverageAudit.#LINE_BREAK);
  }

  /** @param {FileMeasurement[]} files @returns {{report:string,classes:number,methods:number}} */
  #summary(files) {
    return { report: this.#output, classes: files.length, methods: this.#methodCount(files) };
  }

  /** @param {FileMeasurement[]} files @returns {number} */
  #methodCount(files) {
    return files.reduce(/** @param {number} total @param {FileMeasurement} file @returns {number} */
      (total, file) => total + file.methods.length, 0);
  }

  /** @param {string[]} values @returns {{parser:string,directory:string,output:string}} */
  static #configuration(values) {
    if (values.length !== ScopedCoverageAudit.#ARGUMENT_COUNT) throw new Error('Provide existing TypeScript AST module, V8 coverage directory and JSON report path.');
    return { parser: values[0], directory: values[1], output: values[2] };
  }

  /** @returns {CoverageProfile[]} */
  #profiles() {
    const names = readdirSync(this.#directory).filter(/** @param {string} name @returns {boolean} */ name => name.endsWith('.json'));
    this.#require(names.length <= ScopedCoverageAudit.#MAXIMUM_PROFILES, 'Use a fresh bounded coverage directory.');
    return names.map(/** @param {string} name @returns {CoverageProfile} */ name => this.#profile(name));
  }

  /** @param {string} name @returns {CoverageProfile} */
  #profile(name) {
    const file = resolve(this.#directory, name);
    this.#require(statSync(file).size <= ScopedCoverageAudit.#MAXIMUM_PROFILE_BYTES, 'Coverage profile exceeded the bounded audit size.');
    return JSON.parse(readFileSync(file, ScopedCoverageAudit.#FILE_ENCODING));
  }

  /** @param {CoverageProfile[]} profiles @returns {FileMeasurement[]} */
  #files(profiles) {
    return ScopedCoverageAudit.#FILES.map(/** @param {string} file @returns {FileMeasurement} */ file => this.#file(file, profiles));
  }

  /** @param {string} file @param {CoverageProfile[]} profiles @returns {FileMeasurement} */
  #file(file, profiles) {
    const source = readFileSync(file, ScopedCoverageAudit.#FILE_ENCODING);
    const url = pathToFileURL(resolve(file)).href;
    const scripts = profiles.flatMap(/** @param {CoverageProfile} profile @returns {CoveredScript[]} */ profile => profile.result)
      .filter(/** @param {CoveredScript} script @returns {boolean} */ script => script.url === url);
    this.#require(scripts.length > ScopedCoverageAudit.#EMPTY_COUNT, 'No real VM coverage found for ' + file);
    const tokens = this.#tokens(source);
    const functions = scripts.flatMap(/** @param {CoveredScript} script @returns {CoveredFunction[]} */ script => script.functions)
      .filter(ScopedCoverageAudit.#method);
    const names = [...new Set(functions.map(/** @param {CoveredFunction} method @returns {string} */ method => method.functionName))];
    const methods = names.map(/** @param {string} name @returns {MethodMeasurement} */ name => this.#measurement(name, functions, tokens, source));
    return { file, sha256: createHash('sha256').update(source).digest('hex'), vm_instances: scripts.length, methods };
  }

  /** @param {CoveredFunction} method @returns {boolean} */
  static #method(method) {
    return method.functionName !== '' && !method.functionName.startsWith('<');
  }

  /** @param {string} source @returns {SourceToken[]} */
  #tokens(source) {
    const scanner = this.#parser.createScanner(true, this.#parser.LanguageVariant.Standard, source);
    const tokens = [];
    const deadline = Date.now() + ScopedCoverageAudit.#MAXIMUM_MILLISECONDS;
    let previous = -1;
    while (scanner.scan() !== this.#parser.SyntaxKind.EndOfFile) {
      this.#require(ScopedCoverageAudit.#validToken(scanner, source.length, previous), 'Scanner must make bounded forward progress.');
      this.#require(Date.now() < deadline, 'Scanner exceeded its ten-second budget.');
      previous = scanner.getTokenEnd();
      tokens.push({ text: scanner.getTokenText(), start: scanner.getTokenStart(), end: scanner.getTokenEnd() });
    }
    return tokens;
  }

  /** @param {SourceScanner} scanner @param {number} length @param {number} previous @returns {boolean} */
  static #validToken(scanner, length, previous) {
    return Number.isSafeInteger(scanner.getTokenStart()) && Number.isSafeInteger(scanner.getTokenEnd())
      && scanner.getTokenEnd() > scanner.getTokenStart() && scanner.getTokenEnd() <= length && scanner.getTokenEnd() > previous;
  }

  /** @param {string} name @param {CoveredFunction[]} functions @param {SourceToken[]} tokens @param {string} source @returns {MethodMeasurement} */
  #measurement(name, functions, tokens, source) {
    const instances = functions.filter(/** @param {CoveredFunction} method @returns {boolean} */ method => method.functionName === name);
    const bounds = instances[0].ranges[0];
    this.#require(instances.every(/** @param {CoveredFunction} method @returns {boolean} */
      method => method.ranges[0].startOffset === bounds.startOffset && method.ranges[0].endOffset === bounds.endOffset), 'Unstable VM source offsets for ' + name);
    const methodTokens = tokens.filter(/** @param {SourceToken} token @returns {boolean} */ token => token.start >= bounds.startOffset && token.end <= bounds.endOffset);
    const executable = methodTokens.filter(/** @param {SourceToken} token @returns {boolean} */ token => !ScopedCoverageAudit.#PUNCTUATION.has(token.text));
    const coverage = this.#coverage(executable, instances, source);
    const complexity = 1 + methodTokens.filter(/** @param {SourceToken} token @returns {boolean} */ token => ScopedCoverageAudit.#DECISIONS.has(token.text)).length;
    const fraction = coverage.covered_lines / coverage.total_lines;
    const score = complexity * complexity * Math.pow(1 - fraction, ScopedCoverageAudit.#EXPONENT) + complexity;
    return { name, start_line: ScopedCoverageAudit.#line(source, bounds.startOffset), end_line: ScopedCoverageAudit.#line(source, bounds.endOffset), calls: instances.reduce(
      /** @param {number} total @param {CoveredFunction} method @returns {number} */ (total, method) => total + method.ranges[0].count, 0), syntactic_complexity: complexity, ...coverage, crap: score };
  }

  /** @param {SourceToken[]} tokens @param {CoveredFunction[]} instances @param {string} source @returns {CoverageMeasurement} */
  #coverage(tokens, instances, source) {
    this.#require(tokens.length > ScopedCoverageAudit.#EMPTY_COUNT, 'No executable tokens found in covered method.');
    const measured = tokens.map(/** @param {SourceToken} token @returns {MeasuredToken} */ token => ({
      line: ScopedCoverageAudit.#line(source, token.start),
      covered: instances.some(/** @param {CoveredFunction} method @returns {boolean} */ method => ScopedCoverageAudit.#covered(token, method.ranges))
    }));
    const lines = [...new Set(measured.map(/** @param {MeasuredToken} token @returns {number} */ token => token.line))];
    const coveredLines = lines.filter(/** @param {number} line @returns {boolean} */ line => measured.some(
      /** @param {MeasuredToken} token @returns {boolean} */ token => token.line === line && token.covered));
    const coveredTokens = measured.filter(/** @param {MeasuredToken} token @returns {boolean} */ token => token.covered).length;
    return { total_lines: lines.length, covered_lines: coveredLines.length, line_coverage: coveredLines.length / lines.length, total_tokens: tokens.length, covered_tokens: coveredTokens, token_coverage: coveredTokens / tokens.length };
  }

  /** @param {SourceToken} token @param {import('node:inspector').Profiler.CoverageRange[]} ranges @returns {boolean} */
  static #covered(token, ranges) {
    const containing = ranges.filter(/** @param {import('node:inspector').Profiler.CoverageRange} range @returns {boolean} */
      range => range.startOffset <= token.start && range.endOffset >= token.end);
    containing.sort(/** @param {import('node:inspector').Profiler.CoverageRange} first @param {import('node:inspector').Profiler.CoverageRange} second @returns {number} */
      (first, second) => (first.endOffset - first.startOffset) - (second.endOffset - second.startOffset));
    return containing[0].count > ScopedCoverageAudit.#EMPTY_COUNT;
  }

  /** @param {string} source @param {number} offset @returns {number} */
  static #line(source, offset) {
    return source.slice(0, offset).split(ScopedCoverageAudit.#LINE_BREAK).length;
  }

  /** @param {boolean} valid @param {string} message @returns {void} */
  #require(valid, message) {
    if (!valid) throw new Error(message);
  }

  /** @param {FileMeasurement[]} files @returns {CoverageReport} */
  #report(files) {
    return {
      measured_at: new Date().toISOString(), node: process.version,
      scope: ScopedCoverageAudit.#FILES,
      parser: 'Existing TypeScript scanner, supplied explicitly; no dependency installation',
      coverage: 'Real V8 block ranges merged across deterministic VM instances; whitespace-preserving test-only export/decorator/import-meta lowering keeps original UTF-16 offsets',
      complexity: 'Syntactic McCabe-style base 1 plus if/for/while/do/case/catch/conditional/short-circuit tokens; strings and comments excluded by the TypeScript scanner',
      line_definition: 'Lines containing non-punctuation tokens in a named V8 method range; covered when at least one such token has positive innermost V8 block count in any fixture instance',
      formula: 'CRAP = complexity^2 * (1 - measured_line_coverage)^3 + complexity',
      limitations: [
        'Scoped scanner-derived line coverage, not Istanbul statement/branch coverage or whole-app coverage',
        'Boolean array validation and calls to branching dependencies are not extra syntactic decisions',
        'Async rejection paths and native provider implementations are not represented by syntactic complexity alone',
        'Anonymous callbacks and static/instance initializers are excluded from per-method scores',
        'Only declared named methods emitted in V8 profiles are measured; public load/read are actually executed through browser/native-boundary fakes',
        'No native WKWebView, production IPC, compiler lowering or system-resource behavior is verified by these tests'
      ],
      files
    };
  }
}

await ScopedCoverageAudit.main(process.argv.slice(ScopedCoverageAudit.ARGUMENT_SKIP));
