export class DomUtils {
  /** @returns {string} */
  static get #SVG_NAMESPACE() {
    return 'http://www.w3.org/2000/svg';
  }

  /** @param {Element} parent @param {string} tag @param {string} text @param {string} className @returns {HTMLElement} */
  static label(parent, tag, text, className = '') {
    const child = document.createElement(tag);
    child.textContent = text;
    child.className = className;
    parent.append(child);
    return child;
  }

  /** @param {string} tag @param {string} className @returns {HTMLElement} */
  static element(tag, className) {
    const element = document.createElement(tag);
    element.className = className;
    return element;
  }

  /** @param {Element} parent @param {string} tag @param {string} className @returns {SVGElement} */
  static svg(parent, tag, className) {
    const element = /** @type {SVGElement} */ (document.createElementNS(DomUtils.#SVG_NAMESPACE, tag));
    element.classList.add(className);
    parent.append(element);
    return element;
  }

}
