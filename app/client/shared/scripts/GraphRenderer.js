import { DomUtils } from '../../utils/DomUtils.js';
import { GraphGroupRenderer } from './GraphGroupRenderer.js';

export class GraphRenderer {
  /** @type {WeakMap<HTMLElement, GraphGroupRenderer>} */
  static #renderers = new WeakMap();
  /** @type {number} */
  static #INITIAL_ZOOM = 1;
  /** @type {number} */
  static #FIT_MINIMUM_ZOOM = 0.001;
  /** @type {number} */
  static #SIZE_MARGIN = 4;
  /** @type {number} */
  static #MAXIMUM_ZOOM = 1.5;
  /** @type {number} */
  static #ZOOM_STEP = 0.1;
  /** @type {number} */
  static #MAXIMUM_GROUP_SETTINGS = 256;
  /** @type {number} */
  static #NO_SIZE = 0;
  /** @type {string} */
  static #NODE_SELECTOR = '.graph-node';
  /** @type {string} */
  static #SCROLL_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static #GROUP_SELECTOR = '.graph-group';
  /** @type {string} */
  static #FIT_ACTION = 'fit';
  /** @type {string} */
  static #DIV_TAG = 'div';
  /** @type {string} */
  static #ROLE_ATTRIBUTE = 'role';
  /** @type {Map<string, number>} */
  static #zooms = new Map();

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {boolean} force @returns {void} */
  static draw(view, selection, force = false) {
    const stage = view.querySelector('.graph-stage');
    const ready = GraphRenderer.#ready(stage, selection);
    GraphRenderer.#available({view, stage: /** @type {HTMLElement | null} */ (stage), selection, force}, ready);
  }

  /** @param {HTMLElement} view @param {string} identifier @param {string} action @returns {void} */
  static zoom(view, identifier, action) {
    const group = GraphRenderer.#groupElement(view, identifier);
    GraphRenderer.#zoomAvailable(group, identifier, action);
  }

  /** @param {HTMLElement} view @param {string} identifier @returns {HTMLElement | undefined} */
  static #groupElement(view, identifier) {
    return /** @type {HTMLElement | undefined} */ (Array.from(view.querySelectorAll(GraphRenderer.#GROUP_SELECTOR)).find(
      /** @param {Element} element @returns {boolean} */
      element => /** @type {HTMLElement} */ (element).dataset.group === identifier
    ));
  }

  /** @param {HTMLElement | undefined} group @param {string} identifier @param {string} action @returns {void} */
  static #zoomAvailable(group, identifier, action) {
    if (!group) {
      return;
    }
    GraphRenderer.#applyZoom(group, identifier, action);
  }

  /** @param {HTMLElement} group @param {string} identifier @param {string} action @returns {void} */
  static #applyZoom(group, identifier, action) {
    const current = GraphRenderer.#groupZoom(identifier);
    const step = GraphRenderer.#step(action);
    const changed = GraphRenderer.#clamp(current + step);
    const fit = GraphRenderer.#fit(group, current);
    GraphRenderer.#rememberZoom(identifier, GraphRenderer.#selectedZoom(action, changed, fit));
  }

  /** @param {string} identifier @returns {number} */
  static #groupZoom(identifier) {
    return GraphRenderer.#zooms.get(identifier) ?? GraphRenderer.#INITIAL_ZOOM;
  }

  /** @param {number} zoom @returns {number} */
  static #clamp(zoom) {
    return Math.max(GraphRenderer.#FIT_MINIMUM_ZOOM, Math.min(GraphRenderer.#MAXIMUM_ZOOM, zoom));
  }

  /** @param {string} action @param {number} changed @param {number} fit @returns {number} */
  static #selectedZoom(action, changed, fit) {
    if (action === GraphRenderer.#FIT_ACTION) {
      return fit;
    }
    return changed;
  }

  /** @param {HTMLElement} group @param {number} current @returns {number} */
  static #fit(group, current) {
    const scroll = /** @type {HTMLElement | null} */ (group.querySelector(GraphRenderer.#SCROLL_SELECTOR));
    const canvas = /** @type {HTMLElement | null} */ (group.querySelector('.graph-canvas'));
    const canvasSize = GraphRenderer.#canvasSize(canvas);
    const viewportSize = GraphRenderer.#viewportSize(scroll);
    const bounds = {width: canvasSize.width, height: canvasSize.height, availableWidth: viewportSize.width, availableHeight: viewportSize.height};
    const usable = Object.values(bounds).every(
      /** @param {number} value @returns {boolean} */
      value => [Number.isFinite(value), value > GraphRenderer.#NO_SIZE].every(Boolean)
    );
    const fallback = GraphRenderer.#fitWhenUsable(bounds, current, usable);
    const renderer = GraphRenderer.#renderers.get(group);
    const adaptive = GraphRenderer.#adaptiveFit(renderer, bounds, usable);
    return GraphRenderer.#chosenFit(adaptive, fallback);
  }

  /** @param {GraphGroupRenderer | undefined} renderer @param {{availableWidth: number, availableHeight: number}} bounds @param {boolean} usable @returns {number | undefined} */
  static #adaptiveFit(renderer, bounds, usable) {
    if (!usable) {
      return undefined;
    }
    return renderer?.fit(bounds.availableWidth - GraphRenderer.#SIZE_MARGIN, bounds.availableHeight - GraphRenderer.#SIZE_MARGIN);
  }

  /** @param {number | undefined} adaptive @param {number} fallback @returns {number} */
  static #chosenFit(adaptive, fallback) {
    return adaptive ?? fallback;
  }

  /** @param {HTMLElement | null} canvas @returns {{width: number, height: number}} */
  static #canvasSize(canvas) {
    if (!canvas) {
      return {width: GraphRenderer.#NO_SIZE, height: GraphRenderer.#NO_SIZE};
    }
    return {width: Number.parseFloat(canvas.style.width), height: Number.parseFloat(canvas.style.height)};
  }

  /** @param {HTMLElement | null} scroll @returns {{width: number, height: number}} */
  static #viewportSize(scroll) {
    if (!scroll) {
      return {width: GraphRenderer.#NO_SIZE, height: GraphRenderer.#NO_SIZE};
    }
    return {width: scroll.clientWidth, height: scroll.clientHeight};
  }

  /** @param {{width: number, height: number, availableWidth: number, availableHeight: number}} bounds @param {number} current @param {boolean} usable @returns {number} */
  static #fitWhenUsable(bounds, current, usable) {
    if (!usable) {
      return current;
    }
    return Math.max(GraphRenderer.#FIT_MINIMUM_ZOOM, Math.min(1, (bounds.availableWidth - GraphRenderer.#SIZE_MARGIN) / bounds.width, (bounds.availableHeight - GraphRenderer.#SIZE_MARGIN) / bounds.height));
  }

  /** @param {string} identifier @param {number} zoom @returns {void} */
  static #rememberZoom(identifier, zoom) {
    GraphRenderer.#zooms.delete(identifier);
    GraphRenderer.#zooms.set(identifier, zoom);
    GraphRenderer.#trimZooms();
  }

  /** @returns {void} */
  static #trimZooms() {
    while (GraphRenderer.#zooms.size > GraphRenderer.#MAXIMUM_GROUP_SETTINGS) {
      const identifier = /** @type {string} */ (GraphRenderer.#zooms.keys().next().value);
      GraphRenderer.#zooms.delete(identifier);
    }
  }

  /** @param {string} action @returns {number} */
  static #step(action) {
    switch (action) {
      case 'in': return GraphRenderer.#ZOOM_STEP;
      case 'out': return -GraphRenderer.#ZOOM_STEP;
      default: return 0;
    }
  }

  /** @param {Element | null} stage @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {boolean} */
  static #ready(stage, selection) {
    return [stage, selection.model].every(Boolean);
  }

  /** @param {{view: HTMLElement, stage: HTMLElement | null, selection: import('/models/ViewSelection.ts').ViewSelection, force: boolean}} input @param {boolean} ready @returns {void} */
  static #available(input, ready) {
    if (!ready) {
      return;
    }
    GraphRenderer.#prepare(input);
  }

  /** @param {{view: HTMLElement, stage: HTMLElement | null, selection: import('/models/ViewSelection.ts').ViewSelection, force: boolean}} input @returns {void} */
  static #prepare(input) {
    const stage = /** @type {HTMLElement} */ (input.stage);
    const signature = GraphRenderer.#signature(input.selection, stage);
    const refresh = GraphRenderer.#refresh(stage, signature, input.force);
    GraphRenderer.#drawWhenChanged(input.view, stage, input.selection, refresh);
  }

  /** @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {HTMLElement} stage @returns {string} */
  static #signature(selection, stage) {
    const groups = /** @type {import('/models/ObservatoryModel.ts').ObservatoryModel} */ (selection.model).sessions;
    const processes = groups.map(
      /** @param {import('/models/RuntimeView.ts').RuntimeView} group @returns {[string, import('/models/ProcessSnapshot.ts').ProcessSnapshot[]]} */
      group => [group.id, group.processes]
    );
    return JSON.stringify([processes, selection.pid, selection.model?.selected, selection.graphGroup, stage.clientWidth, stage.clientHeight, Array.from(GraphRenderer.#zooms)]);
  }

  /** @param {HTMLElement} stage @param {string} signature @param {boolean} force @returns {boolean} */
  static #refresh(stage, signature, force) {
    const changed = stage.dataset.signature !== signature;
    stage.dataset.signature = signature;
    return [changed, force].some(Boolean);
  }

  /** @param {HTMLElement} view @param {HTMLElement} stage @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {boolean} refresh @returns {void} */
  static #drawWhenChanged(view, stage, selection, refresh) {
    if (!refresh) {
      return;
    }
    GraphRenderer.#render(view, stage, selection);
  }

  /** @param {HTMLElement} view @param {HTMLElement} stage @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  static #render(view, stage, selection) {
    const model = /** @type {import('/models/ObservatoryModel.ts').ObservatoryModel} */ (selection.model);
    const focused = GraphRenderer.#focusedProcess();
    const offsets = GraphRenderer.#offsets(stage);
    const tabOffset = GraphRenderer.#tabOffset(stage);
    const grid = DomUtils.element(GraphRenderer.#DIV_TAG, 'graph-grid');
    const group = GraphRenderer.#activeGroup(model.sessions, selection.graphGroup);
    GraphRenderer.#activeCard(grid, group, selection, GraphRenderer.#layoutWidth(stage));
    const tabs = GraphRenderer.#tabs(model.sessions, group);
    stage.replaceChildren(tabs, grid);
    tabs.scrollLeft = tabOffset;
    GraphRenderer.#restore(stage, offsets);
    GraphRenderer.#focus(stage, focused);
  }

  /** @param {HTMLElement} stage @returns {number} */
  static #tabOffset(stage) {
    return stage.querySelector('.graph-group-tabs')?.scrollLeft ?? 0;
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string | undefined} identifier @returns {import('/models/RuntimeView.ts').RuntimeView | undefined} */
  static #activeGroup(groups, identifier) {
    const found = groups.find(/** @param {import('/models/RuntimeView.ts').RuntimeView} group @returns {boolean} */ group => group.id === identifier);
    return GraphRenderer.#firstGroup(found, groups);
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView | undefined} found @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @returns {import('/models/RuntimeView.ts').RuntimeView | undefined} */
  static #firstGroup(found, groups) {
    return found ?? groups[0];
  }

  /** @param {HTMLElement} grid @param {import('/models/RuntimeView.ts').RuntimeView | undefined} group @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {number} width @returns {void} */
  static #activeCard(grid, group, selection, width) {
    if (!group) {
      return;
    }
    grid.append(GraphRenderer.#groupCard(group, selection, width));
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {import('/models/RuntimeView.ts').RuntimeView | undefined} active @returns {HTMLElement} */
  static #tabs(groups, active) {
    const tabs = DomUtils.element(GraphRenderer.#DIV_TAG, 'graph-group-tabs');
    tabs.setAttribute(GraphRenderer.#ROLE_ATTRIBUTE, 'tablist');
    tabs.setAttribute('aria-label', 'Process groups');
    GraphRenderer.#tabItems(tabs, groups, active?.id);
    return tabs;
  }

  /** @param {HTMLElement} tabs @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string | undefined} active @returns {void} */
  static #tabItems(tabs, groups, active) {
    for (const group of groups) {
      tabs.append(GraphRenderer.#tab(group, active));
    }
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView} group @param {string | undefined} active @returns {HTMLElement} */
  static #tab(group, active) {
    const tab = DomUtils.element('button', 'graph-group-tab');
    tab.textContent = group.name + ' · PID ' + group.pid;
    const selected = group.id === active;
    tab.dataset.graphGroup = group.id;
    tab.id = 'graph-tab-' + group.pid;
    tab.setAttribute(GraphRenderer.#ROLE_ATTRIBUTE, 'tab');
    tab.setAttribute('aria-selected', String(selected));
    tab.setAttribute('aria-controls', 'graph-panel-' + group.pid);
    tab.tabIndex = Number(!selected) * -1;
    return tab;
  }

  /** @param {HTMLElement} stage @returns {number} */
  static #layoutWidth(stage) {
    return stage.querySelector(GraphRenderer.#SCROLL_SELECTOR)?.clientWidth ?? stage.clientWidth - GraphRenderer.#SIZE_MARGIN;
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView} group @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {number} width @returns {HTMLElement} */
  static #groupCard(group, selection, width) {
    const zoom = GraphRenderer.#groupZoom(group.id);
    const renderer = new GraphGroupRenderer(group, selection, zoom, width);
    const element = renderer.render();
    GraphRenderer.#renderers.set(element, renderer);
    return element;
  }

  /** @returns {{group: string | undefined, pid: string | undefined, zoom: string | undefined, graphGroup: string | undefined}} */
  static #focusedProcess() {
    const element = /** @type {HTMLElement | null} */ (document.activeElement);
    return {group: element?.dataset.group, pid: element?.dataset.pid, zoom: element?.dataset.zoom, graphGroup: element?.dataset.graphGroup};
  }

  /** @param {HTMLElement} stage @returns {Map<string | undefined, {top: number, left: number}>} */
  static #offsets(stage) {
    return new Map(Array.from(stage.querySelectorAll(GraphRenderer.#SCROLL_SELECTOR), GraphRenderer.#elementOffset));
  }

  /** @param {Element} element @returns {[string | undefined, {top: number, left: number}]} */
  static #elementOffset(element) {
    return [/** @type {HTMLElement} */ (element).dataset.group, {top: element.scrollTop, left: element.scrollLeft}];
  }

  /** @param {HTMLElement} stage @param {Map<string | undefined, {top: number, left: number}>} offsets @returns {void} */
  static #restore(stage, offsets) {
    for (const element of stage.querySelectorAll(GraphRenderer.#SCROLL_SELECTOR)) {
      GraphRenderer.#restoreElement(/** @type {HTMLElement} */ (element), offsets);
    }
  }

  /** @param {HTMLElement} element @param {Map<string | undefined, {top: number, left: number}>} offsets @returns {void} */
  static #restoreElement(element, offsets) {
    const center = GraphRenderer.#rootCenter(element);
    const initial = {top: 0, left: Math.max(0, center - element.clientWidth / 2)};
    const offset = GraphRenderer.#savedOffset(offsets.get(element.dataset.group), initial);
    element.scrollTop = offset.top;
    element.scrollLeft = offset.left;
  }

  /** @param {HTMLElement} element @returns {number} */
  static #rootCenter(element) {
    return Number(element.dataset.rootCenter ?? 0);
  }

  /** @param {{top: number, left: number} | undefined} saved @param {{top: number, left: number}} initial @returns {{top: number, left: number}} */
  static #savedOffset(saved, initial) {
    return saved ?? initial;
  }

  /** @param {HTMLElement} stage @param {{group: string | undefined, pid: string | undefined, zoom: string | undefined, graphGroup: string | undefined}} focused @returns {void} */
  static #focus(stage, focused) {
    const node = Array.from(stage.querySelectorAll(GraphRenderer.#NODE_SELECTOR + ', [data-zoom], [data-graph-group], ' + GraphRenderer.#SCROLL_SELECTOR)).find(
      /** @param {Element} element @returns {boolean} */
      element => GraphRenderer.#matchingFocus(/** @type {HTMLElement} */ (element), focused)
    );
    /** @type {HTMLElement | undefined} */ (node)?.focus({preventScroll: true});
  }

  /** @param {HTMLElement} element @param {{group: string | undefined, pid: string | undefined, zoom: string | undefined, graphGroup: string | undefined}} focused @returns {boolean} */
  static #matchingFocus(element, focused) {
    return [element.dataset.group === focused.group, element.dataset.pid === focused.pid, element.dataset.zoom === focused.zoom, element.dataset.graphGroup === focused.graphGroup].every(Boolean);
  }

}
