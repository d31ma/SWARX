@Service
export default class GraphService {
  /** @type {number} */
  static _CARD_WIDTH = 260;
  /** @type {number} */
  static _CARD_HEIGHT = 160;
  /** @type {number} */
  static _GAP = 24;
  /** @type {number} */
  static _DEPTH_GAP = 64;
  /** @type {number} */
  static _CURVE = 16;
  /** @type {number} */
  static _PADDING = 8;
  /** @type {number} */
  static _MINIMUM_ZOOM = 0.001;
  /** @type {number} */
  static _MAXIMUM_ZOOM = 1.5;
  /** @type {number} */
  static _ZOOM_STEP = 0.1;
  /** @type {number} */
  static _MAXIMUM_SETTINGS = 256;
  /** @type {string} */
  static _GROUP_SELECTOR = '.graph-group';
  /** @type {string} */
  static _NODE_SELECTOR = '.graph-node';
  /** @type {string} */
  static _VIEWPORT_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static _EMPTY = '';
  /** @type {Map<string, number>} */
  #zooms = new Map();

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {boolean} _force @returns {void} */
  draw(view, selection, _force = false) {
    this.#tableSelection(view, selection);
    const groups = this.#visibleGroups(view);
    this.#drawGroups(groups, selection);
  }

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #tableSelection(view, selection) {
    for (const element of view.querySelectorAll('.session-process-table .session-process-pick')) {
      const node = /** @type {HTMLElement} */ (element);
      const selected = [node.dataset.session === selection.session, Number(node.dataset.pid) === selection.sessionPid, node.dataset.started === selection.sessionStarted].every(Boolean);
      node.classList.toggle('selected', selected);
    }
  }

  /** @param {HTMLElement} view @param {string} identifier @param {string} action @returns {void} */
  zoom(view, identifier, action) {
    const groups = this.#matchingGroups(view, identifier);
    this.#changeGroups(groups, action);
  }

  /** @returns {void} */
  @OnStop
  dispose() {
    this.#zooms.clear();
  }

  /** @param {HTMLElement} view @returns {HTMLElement[]} */
  #visibleGroups(view) {
    return /** @type {HTMLElement[]} */ (Array.from(view.querySelectorAll(GraphService._GROUP_SELECTOR))).filter(
      /** @param {HTMLElement} group @returns {boolean} */ group => this.#viewport(group).width > 0
    );
  }

  /** @param {HTMLElement} view @param {string} identifier @returns {HTMLElement[]} */
  #matchingGroups(view, identifier) {
    return this.#visibleGroups(view).filter(
      /** @param {HTMLElement} group @returns {boolean} */ group => group.dataset.group === identifier
    );
  }

  /** @param {HTMLElement[]} groups @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #drawGroups(groups, selection) {
    for (const group of groups) {
      this.#drawGroup(group, selection);
    }
  }

  /** @param {HTMLElement[]} groups @param {string} action @returns {void} */
  #changeGroups(groups, action) {
    for (const group of groups) {
      this.#changeZoom(group, action);
    }
  }

  /** @param {HTMLElement} group @returns {{width: number, height: number}} */
  #viewport(group) {
    const viewport = /** @type {HTMLElement | null} */ (group.querySelector(GraphService._VIEWPORT_SELECTOR));
    return {width: viewport?.clientWidth ?? 0, height: viewport?.clientHeight ?? 0};
  }

  /** @param {HTMLElement} group @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #drawGroup(group, selection) {
    const layout = this.#layout(group);
    const zoom = this.#zoom(group.dataset.group);
    this.#size(group, layout, zoom);
    this.#placeNodes(group, layout, selection);
    this.#trunks(group, layout);
    this.#branches(group, layout);
  }

  /** @param {HTMLElement} group @returns {import('/models/GraphLayout.ts').GraphLayout} */
  #layout(group) {
    const width = Math.max(1, this.#viewport(group).width - GraphService._PADDING * 2);
    const nodes = /** @type {HTMLElement[]} */ (Array.from(group.querySelectorAll(GraphService._NODE_SELECTOR)));
    const columns = Math.max(1, Math.floor((width + GraphService._GAP) / (GraphService._CARD_WIDTH + GraphService._GAP)));
    const cardWidth = Math.min(GraphService._CARD_WIDTH, width);
    this.#nodeWidths(nodes, cardWidth);
    const levels = this.#levels(nodes);
    return this.#placeLevels(levels, {width, columns, cardWidth});
  }

  /** @param {HTMLElement[]} nodes @param {number} width @returns {void} */
  #nodeWidths(nodes, width) {
    for (const node of nodes) {
      node.style.width = width + 'px';
    }
  }

  /** @param {HTMLElement[]} nodes @returns {Map<number, HTMLElement[]>} */
  #levels(nodes) {
    /** @type {Map<number, HTMLElement[]>} */
    const levels = new Map();
    for (const node of nodes) {
      const depth = Number(node.dataset.depth);
      const level = levels.get(depth) ?? [];
      level.push(node);
      levels.set(depth, level);
    }
    return levels;
  }

  /** @param {Map<number, HTMLElement[]>} levels @param {{width: number, columns: number, cardWidth: number}} size @returns {import('/models/GraphLayout.ts').GraphLayout} */
  #placeLevels(levels, size) {
    /** @type {import('/models/GraphLayout.ts').GraphLayout} */
    const layout = {boxes: new Map(), width: size.width, height: GraphService._PADDING};
    const depths = Array.from(levels.keys()).sort(/** @param {number} left @param {number} right @returns {number} */ (left, right) => left - right);
    for (const depth of depths) {
      const nodes = /** @type {HTMLElement[]} */ (levels.get(depth));
      this.#placeRows(nodes, size, layout);
      layout.height += GraphService._DEPTH_GAP;
    }
    layout.height = Math.max(GraphService._PADDING, layout.height - GraphService._DEPTH_GAP + GraphService._PADDING);
    return layout;
  }

  /** @param {HTMLElement[]} nodes @param {{width: number, columns: number, cardWidth: number}} size @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #placeRows(nodes, size, layout) {
    for (let offset = 0; offset < nodes.length; offset += size.columns) {
      const row = nodes.slice(offset, offset + size.columns);
      this.#placeRow(row, size, layout);
    }
  }

  /** @param {HTMLElement[]} row @param {{width: number, cardWidth: number}} size @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #placeRow(row, size, layout) {
    const width = row.length * size.cardWidth + (row.length - 1) * GraphService._GAP;
    const left = (size.width - width) / 2;
    const height = Math.max(GraphService._CARD_HEIGHT, ...row.map(
      /** @param {HTMLElement} node @returns {number} */ node => node.offsetHeight
    ));
    this.#rowBoxes(row, {left, top: layout.height, width: size.cardWidth, height}, layout);
    layout.height += height + GraphService._GAP;
  }

  /** @param {HTMLElement[]} row @param {{left: number, top: number, width: number, height: number}} position @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #rowBoxes(row, position, layout) {
    for (let index = 0; index < row.length; index += 1) {
      const element = row[index];
      const identity = element.dataset.identity ?? GraphService._EMPTY;
      layout.boxes.set(identity, {element, identity, left: position.left + index * (position.width + GraphService._GAP), top: position.top, width: position.width, height: position.height});
    }
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {number} zoom @returns {void} */
  #size(group, layout, zoom) {
    const canvas = /** @type {HTMLElement | null} */ (group.querySelector('.graph-canvas'));
    const size = /** @type {HTMLElement | null} */ (group.querySelector('.group-graph-size'));
    const svg = /** @type {SVGElement | null} */ (group.querySelector('.graph-edges'));
    const label = group.querySelector('.graph-zoom-value');
    this.#canvasSize(canvas, layout, zoom);
    this.#scaledSize(size, layout, zoom);
    svg?.setAttribute('viewBox', '0 0 ' + layout.width + ' ' + layout.height);
    svg?.setAttribute('width', String(layout.width));
    svg?.setAttribute('height', String(layout.height));
    this.#zoomLabel(label, zoom);
  }

  /** @param {HTMLElement | null} canvas @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {number} zoom @returns {void} */
  #canvasSize(canvas, layout, zoom) {
    if (!canvas) return;
    Object.assign(canvas.style, {width: layout.width + 'px', height: layout.height + 'px', transform: 'scale(' + zoom + ')'});
  }

  /** @param {HTMLElement | null} element @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {number} zoom @returns {void} */
  #scaledSize(element, layout, zoom) {
    if (!element) return;
    Object.assign(element.style, {width: layout.width * zoom + 'px', height: layout.height * zoom + 'px'});
  }

  /** @param {Element | null} element @param {number} zoom @returns {void} */
  #zoomLabel(element, zoom) {
    if (!element) return;
    element.textContent = Math.round(zoom * 100) + '%';
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #placeNodes(group, layout, selection) {
    for (const box of layout.boxes.values()) {
      Object.assign(box.element.style, {left: box.left + 'px', top: box.top + 'px', minHeight: box.height + 'px'});
      box.element.classList.toggle('selected', this.#selected(group, box.element, selection));
    }
  }

  /** @param {HTMLElement} group @param {HTMLElement} node @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {boolean} */
  #selected(group, node, selection) {
    return [group.dataset.session === selection.session, Number(node.dataset.pid) === selection.sessionPid, node.dataset.started === selection.sessionStarted].every(Boolean);
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #trunks(group, layout) {
    for (const path of group.querySelectorAll('.shared-trunk')) {
      this.#trunk(/** @type {SVGElement} */ (path), group, layout);
    }
  }

  /** @param {SVGElement} path @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #trunk(path, group, layout) {
    const parent = layout.boxes.get(path.dataset.parentIdentity ?? GraphService._EMPTY);
    const children = this.#children(group, path.dataset.parentIdentity, layout);
    const available = [parent !== undefined, children.length > 0].every(Boolean);
    const value = this.#trunkPath(parent, children, available);
    path.setAttribute('d', value);
  }

  /** @param {HTMLElement} group @param {string | undefined} identity @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} */
  #children(group, identity, layout) {
    const paths = /** @type {SVGElement[]} */ (Array.from(group.querySelectorAll('.child-branch')));
    return paths.filter(/** @param {SVGElement} path @returns {boolean} */ path => path.dataset.parentIdentity === identity)
      .map(/** @param {SVGElement} path @returns {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} */ path => layout.boxes.get(path.dataset.childIdentity ?? GraphService._EMPTY))
      .filter(/** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} box @returns {box is import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ box => box !== undefined);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @param {boolean} available @returns {string} */
  #trunkPath(parent, children, available) {
    if (!available) return GraphService._EMPTY;
    const root = /** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (parent);
    const centers = children.map(/** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} box @returns {number} */ box => this.#branchStart(root, box));
    const x = root.left + root.width / 2;
    const y = root.top + root.height;
    const branch = y + GraphService._DEPTH_GAP / 2;
    return 'M ' + x + ' ' + y + ' V ' + branch + ' M ' + Math.min(x, ...centers) + ' ' + branch + ' H ' + Math.max(x, ...centers);
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #branches(group, layout) {
    for (const path of group.querySelectorAll('.child-branch')) {
      this.#branch(/** @type {SVGElement} */ (path), layout);
    }
  }

  /** @param {SVGElement} path @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #branch(path, layout) {
    const parent = layout.boxes.get(path.dataset.parentIdentity ?? GraphService._EMPTY);
    const child = layout.boxes.get(path.dataset.childIdentity ?? GraphService._EMPTY);
    const available = [parent !== undefined, child !== undefined].every(Boolean);
    path.setAttribute('d', this.#branchPath(parent, child, available));
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} child @param {boolean} available @returns {string} */
  #branchPath(parent, child, available) {
    if (!available) return GraphService._EMPTY;
    const root = /** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (parent);
    const target = /** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (child);
    const x = target.left + target.width / 2;
    const y = root.top + root.height + GraphService._DEPTH_GAP / 2;
    const start = this.#branchStart(root, target);
    const curve = Math.abs(start - x);
    return 'M ' + start + ' ' + y + ' Q ' + x + ' ' + y + ' ' + x + ' ' + (y + curve) + ' V ' + target.top;
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @returns {number} */
  #branchStart(parent, child) {
    const x = child.left + child.width / 2;
    const distance = parent.left + parent.width / 2 - x;
    return x + Math.sign(distance) * Math.min(GraphService._CURVE, Math.abs(distance));
  }

  /** @param {string | undefined} identifier @returns {number} */
  #zoom(identifier) {
    return this.#zooms.get(identifier ?? GraphService._EMPTY) ?? 1;
  }

  /** @param {HTMLElement} group @param {string} action @returns {void} */
  #changeZoom(group, action) {
    const identifier = group.dataset.group ?? GraphService._EMPTY;
    const current = this.#zoom(identifier);
    const fit = this.#fit(group);
    const zoom = this.#chosenZoom(action, current, fit);
    this.#zooms.delete(identifier);
    this.#zooms.set(identifier, zoom);
    this.#trim();
  }

  /** @param {HTMLElement} group @returns {number} */
  #fit(group) {
    const layout = this.#layout(group);
    const viewport = this.#viewport(group);
    return this.#clamp(Math.min(1, (viewport.width - GraphService._PADDING) / layout.width, (viewport.height - GraphService._PADDING) / layout.height));
  }

  /** @param {string} action @param {number} current @param {number} fit @returns {number} */
  #chosenZoom(action, current, fit) {
    switch (action) {
      case 'fit': return fit;
      case 'in': return this.#clamp(current + GraphService._ZOOM_STEP);
      case 'out': return this.#clamp(current - GraphService._ZOOM_STEP);
      default: return current;
    }
  }

  /** @param {number} value @returns {number} */
  #clamp(value) {
    return Math.max(GraphService._MINIMUM_ZOOM, Math.min(GraphService._MAXIMUM_ZOOM, value));
  }

  /** @returns {void} */
  #trim() {
    while (this.#zooms.size > GraphService._MAXIMUM_SETTINGS) {
      this.#zooms.delete(/** @type {string} */ (this.#zooms.keys().next().value));
    }
  }
}
