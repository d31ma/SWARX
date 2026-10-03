export class ScriptUtils {
  /** @returns {string} */
  static get #THEME_KEY() {
    return 'w-theme';
  }
  /** @returns {string} */
  static get #DEFAULT_THEME() {
    return 'dark';
  }
  /** @returns {void} */
  static load() {
    ScriptUtils.#theme();
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
