export class ScriptUtils {
  /** @returns {string} */
  static get #THEME_KEY() {
    return 'w-theme';
  }
  /** @returns {string} */
  static get #DEFAULT_THEME() {
    return 'dark';
  }
  /** @returns {{file: string, marker: string}[]} */
  static get #SCRIPTS() {
    return [
      {file: 'duvay/duvay-wc.min.js', marker: 'wc'},
      {file: 'duvay/duvay.min.js', marker: 'behaviours'},
      {file: 'chartjs/chart.umd.min.js', marker: 'charts'}
    ];
  }

  /** @returns {void} */
  static load() {
    ScriptUtils.#scripts();
    ScriptUtils.#theme();
  }

  /** @returns {void} */
  static #scripts() {
    for (const entry of ScriptUtils.#SCRIPTS) {
      ScriptUtils.#script(entry);
    }
  }

  /** @param {{file: string, marker: string}} entry @returns {void} */
  static #script(entry) {
    if (document.querySelector('script[data-observatory="' + entry.marker + '"]')) {
      return;
    }
    const script = document.createElement('script');
    script.src = new URL('../shared/assets/' + entry.file, import.meta.url).href;
    script.defer = true;
    script.dataset.observatory = entry.marker;
    document.head.append(script);
  }

  /** @returns {void} */
  static #theme() {
    try {
      const stored = localStorage.getItem(ScriptUtils.#THEME_KEY);
      ScriptUtils.#applyStoredTheme(stored);
    } catch {
      document.documentElement.setAttribute(ScriptUtils.#THEME_KEY, ScriptUtils.#DEFAULT_THEME);
    }
  }

  /** @param {string | null} stored @returns {string} */
  static #defaultTheme(stored) {
    return stored ?? ScriptUtils.#DEFAULT_THEME;
  }

  /** @param {string | null} stored @returns {void} */
  static #applyStoredTheme(stored) {
    const theme = ScriptUtils.#defaultTheme(stored);
    localStorage.setItem(ScriptUtils.#THEME_KEY, theme);
    document.documentElement.setAttribute(ScriptUtils.#THEME_KEY, theme);
  }
}
