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
  /** @type {number} */
  static _SINGLE_ROW = 1;
  /** @type {number} */
  static _PADDING_SIDE_COUNT = 2;
  /** @type {number} */
  static _MIDPOINT_DIVISOR = 2;
  /** @type {Map<string, number>} */
  #zooms = new Map();
  /** @type {string} */
  static _GROUP_SELECTOR = '.graph-group';
  /** @type {string} */
  static _NODE_SELECTOR = '.graph-node';
  /** @type {string} */
  static _VIEWPORT_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static _EMPTY = '';
  /** @type {string} */
  static _SVG_COORDINATE_SEPARATOR = ' ';
  /** @type {string} */
  static _SVG_MOVE_COMMAND = 'M ';
  /** @type {string} */
  static _SVG_HORIZONTAL_COMMAND = ' H ';
  /** @type {string} */
  static _SVG_VERTICAL_COMMAND = ' V ';
  /** @type {string} */
  static _SVG_QUADRATIC_COMMAND = ' Q ';
  /** @type {string} */
  static _CHILD_BRANCH_SELECTOR = '.child-branch';
  /** @type {string} */
  static _PATH_ATTRIBUTE = 'd';
  /** @type {string} */
  static _WIDTH_ATTRIBUTE = 'width';
  /** @type {string} */
  static _HEIGHT_ATTRIBUTE = 'height';
  /** @type {string} */
  static _SELECTED_CLASS = 'selected';

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {boolean} _force @returns {void} */
  draw(view, selection, _force = false) {
    const groups = this.#visibleGroups(view);
    this.#drawGroups(groups, selection);
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
    return this.#viewportSize(viewport);
  }

  /** @param {HTMLElement | null} viewport @returns {{width: number, height: number}} */
  #viewportSize(viewport) {
    if (viewport === null) {
      return {width: 0, height: 0};
    }
    return {width: viewport.clientWidth, height: viewport.clientHeight};
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
    const width = Math.max(1, this.#viewport(group).width - GraphService._PADDING * GraphService._PADDING_SIDE_COUNT);
    const nodes = /** @type {HTMLElement[]} */ (Array.from(group.querySelectorAll(GraphService._NODE_SELECTOR)));
    const contentWidth = Math.max(1, width - GraphService._PADDING * GraphService._PADDING_SIDE_COUNT);
    const columns = Math.max(1, Math.floor((contentWidth + GraphService._GAP) / (GraphService._CARD_WIDTH + GraphService._GAP)));
    const cardWidth = Math.min(GraphService._CARD_WIDTH, contentWidth);
    this.#nodeWidths(nodes, cardWidth);
    const levels = this.#levels(nodes);
    return this.#placeLevels(levels, {width, columns, cardWidth});
  }

  /** @param {HTMLElement[]} nodes @param {number} width @returns {void} */
  #nodeWidths(nodes, width) {
    for (const node of nodes) {
      node.parentElement?.setAttribute(GraphService._WIDTH_ATTRIBUTE, String(width));
      node.parentElement?.setAttribute(GraphService._HEIGHT_ATTRIBUTE, '0');
    }
  }

  /** @param {HTMLElement[]} nodes @returns {Map<number, HTMLElement[]>} */
  #levels(nodes) {
    /** @type {Map<number, HTMLElement[]>} */
    const levels = new Map();
    for (const node of nodes) {
      this.#addLevel(levels, node);
    }
    return levels;
  }

  /** @param {Map<number, HTMLElement[]>} levels @param {HTMLElement} node @returns {void} */
  #addLevel(levels, node) {
    const depth = Number(node.dataset.depth);
    const level = this.#levelNodes(levels.get(depth));
    level.push(node);
    levels.set(depth, level);
  }

  /** @param {HTMLElement[] | undefined} nodes @returns {HTMLElement[]} */
  #levelNodes(nodes) {
    return nodes ?? [];
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
    const left = (size.width - width) / GraphService._MIDPOINT_DIVISOR;
    const height = Math.max(GraphService._CARD_HEIGHT, ...row.map(
      /** @param {HTMLElement} node @returns {number} */ node => node.offsetHeight
    ));
    this.#rowBoxes(row, {left, top: layout.height, width: size.cardWidth, height}, layout);
    layout.height += height + GraphService._GAP;
  }

  /** @param {HTMLElement[]} row @param {{left: number, top: number, width: number, height: number}} position @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #rowBoxes(row, position, layout) {
    for (let index = 0; index < row.length; index += 1) {
      this.#rowBox(row[index], index, position, layout);
    }
  }

  /** @param {HTMLElement} element @param {number} index @param {{left: number, top: number, width: number, height: number}} position @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #rowBox(element, index, position, layout) {
    const identity = this.#identifier(element.dataset.identity);
    layout.boxes.set(identity, {element, identity, left: position.left + index * (position.width + GraphService._GAP), top: position.top, width: position.width, height: position.height});
  }

  /** @param {string | undefined} identifier @returns {string} */
  #identifier(identifier) {
    return identifier ?? GraphService._EMPTY;
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {number} zoom @returns {void} */
  #size(group, layout, zoom) {
    const svg = group.querySelector('.graph-canvas');
    const label = group.querySelector('.graph-zoom-value');
    svg?.setAttribute('viewBox', '0 0 ' + layout.width + GraphService._SVG_COORDINATE_SEPARATOR + layout.height);
    svg?.setAttribute(GraphService._WIDTH_ATTRIBUTE, String(layout.width * zoom));
    svg?.setAttribute(GraphService._HEIGHT_ATTRIBUTE, String(layout.height * zoom));
    this.#zoomLabel(label, zoom);
  }

  /** @param {Element | null} element @param {number} zoom @returns {void} */
  #zoomLabel(element, zoom) {
    // Rewriting unchanged text replaces a DOM node, which the view continuity observer answers with
    // another draw: an endless redraw loop that reset the tree scroll position on every pass.
    const text = Math.round(zoom * 100) + '%';
    if ([element === null, element?.textContent === text].some(Boolean)) {
      return;
    }
    /** @type {HTMLElement} */ (element).textContent = text;
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #placeNodes(group, layout, selection) {
    for (const box of layout.boxes.values()) {
      box.element.parentElement?.setAttribute('x', String(box.left));
      box.element.parentElement?.setAttribute('y', String(box.top));
      box.element.parentElement?.setAttribute(GraphService._HEIGHT_ATTRIBUTE, String(box.height));
      box.element.classList.toggle(GraphService._SELECTED_CLASS, this.#selected(group, box.element, selection));
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
    const parent = layout.boxes.get(this.#identifier(path.dataset.parentIdentity));
    const children = this.#children(group, path.dataset.parentIdentity, layout);
    const available = [parent !== undefined, children.length > 0].every(Boolean);
    const value = this.#trunkPath(parent, children, {width: layout.width}, available);
    path.setAttribute(GraphService._PATH_ATTRIBUTE, value);
  }

  /** @param {HTMLElement} group @param {string | undefined} identity @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} */
  #children(group, identity, layout) {
    const paths = /** @type {SVGElement[]} */ (Array.from(group.querySelectorAll(GraphService._CHILD_BRANCH_SELECTOR)));
    return paths.filter(/** @param {SVGElement} path @returns {boolean} */ path => path.dataset.parentIdentity === identity)
      .map(/** @param {SVGElement} path @returns {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} */ path => layout.boxes.get(this.#identifier(path.dataset.childIdentity)))
      .filter(/** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} box @returns {box is import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ box => box !== undefined);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @param {{width: number}} size @param {boolean} available @returns {string} */
  #trunkPath(parent, children, size, available) {
    if (!available) return GraphService._EMPTY;
    return this.#trunkRouting(/** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (parent), children, size);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @param {{width: number}} size @returns {string} */
  #trunkRouting(parent, children, size) {
    const route = this.#route(parent, children, size);
    return this.#trunkShape(parent, children, route);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @param {{width: number}} size @returns {{side: boolean, x: number, last: number}} */
  #route(parent, children, size) {
    const tops = children.map(/** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} box @returns {number} */ box => box.top);
    const rows = new Set(tops);
    const distant = Math.min(...tops) - parent.top - parent.height > GraphService._GAP + GraphService._DEPTH_GAP;
    const side = [rows.size > GraphService._SINGLE_ROW, distant].some(Boolean);
    return {side, x: Math.min(GraphService._PADDING / GraphService._MIDPOINT_DIVISOR, size.width / GraphService._MIDPOINT_DIVISOR), last: Math.max(...tops) - GraphService._CURVE};
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @param {{side: boolean, x: number, last: number}} route @returns {string} */
  #trunkShape(parent, children, route) {
    if (route.side) return this.#sideTrunk(parent, route);
    return this.#flatTrunk(parent, children);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {{x: number, last: number}} route @returns {string} */
  #sideTrunk(parent, route) {
    const x = parent.left + parent.width / GraphService._MIDPOINT_DIVISOR;
    const y = parent.top + parent.height;
    const turn = y + GraphService._GAP / GraphService._MIDPOINT_DIVISOR;
    return GraphService._SVG_MOVE_COMMAND + x + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_VERTICAL_COMMAND + turn + GraphService._SVG_HORIZONTAL_COMMAND + route.x + GraphService._SVG_VERTICAL_COMMAND + route.last;
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox[]} children @returns {string} */
  #flatTrunk(parent, children) {
    const centers = children.map(/** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} box @returns {number} */ box => this.#branchStart(parent, box));
    const x = parent.left + parent.width / GraphService._MIDPOINT_DIVISOR;
    const y = parent.top + parent.height;
    const branch = y + GraphService._DEPTH_GAP / GraphService._MIDPOINT_DIVISOR;
    return GraphService._SVG_MOVE_COMMAND + x + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_VERTICAL_COMMAND + branch + ' M ' + Math.min(x, ...centers) + GraphService._SVG_COORDINATE_SEPARATOR + branch + GraphService._SVG_HORIZONTAL_COMMAND + Math.max(x, ...centers);
  }

  /** @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #branches(group, layout) {
    for (const path of group.querySelectorAll(GraphService._CHILD_BRANCH_SELECTOR)) {
      this.#branch(/** @type {SVGElement} */ (path), group, layout);
    }
  }

  /** @param {SVGElement} path @param {HTMLElement} group @param {import('/models/GraphLayout.ts').GraphLayout} layout @returns {void} */
  #branch(path, group, layout) {
    const parent = layout.boxes.get(this.#identifier(path.dataset.parentIdentity));
    const child = layout.boxes.get(this.#identifier(path.dataset.childIdentity));
    const available = [parent !== undefined, child !== undefined].every(Boolean);
    const children = this.#children(group, path.dataset.parentIdentity, layout);
    path.setAttribute(GraphService._PATH_ATTRIBUTE, this.#branchPath(parent, child, {children, width: layout.width}, available));
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox | undefined} child @param {{children: import('/models/GraphLayoutBox.ts').GraphLayoutBox[], width: number}} family @param {boolean} available @returns {string} */
  #branchPath(parent, child, family, available) {
    if (!available) return GraphService._EMPTY;
    return this.#branchRouting(/** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (parent),
      /** @type {import('/models/GraphLayoutBox.ts').GraphLayoutBox} */ (child), family);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @param {{children: import('/models/GraphLayoutBox.ts').GraphLayoutBox[], width: number}} family @returns {string} */
  #branchRouting(parent, child, family) {
    const route = this.#route(parent, family.children, family);
    return this.#branchShape(parent, child, route);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @param {{side: boolean, x: number}} route @returns {string} */
  #branchShape(parent, child, route) {
    if (route.side) return this.#sideBranch(child, route);
    return this.#flatBranch(parent, child);
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @param {{x: number}} route @returns {string} */
  #sideBranch(child, route) {
    const x = child.left + child.width / GraphService._MIDPOINT_DIVISOR;
    const y = child.top - GraphService._CURVE;
    return GraphService._SVG_MOVE_COMMAND + route.x + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_HORIZONTAL_COMMAND + (x - GraphService._CURVE) + GraphService._SVG_QUADRATIC_COMMAND + x + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_COORDINATE_SEPARATOR + x + GraphService._SVG_COORDINATE_SEPARATOR + child.top;
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @returns {string} */
  #flatBranch(parent, child) {
    const x = child.left + child.width / GraphService._MIDPOINT_DIVISOR;
    const y = parent.top + parent.height + GraphService._DEPTH_GAP / GraphService._MIDPOINT_DIVISOR;
    const start = this.#branchStart(parent, child);
    const curve = Math.abs(start - x);
    return GraphService._SVG_MOVE_COMMAND + start + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_QUADRATIC_COMMAND + x + GraphService._SVG_COORDINATE_SEPARATOR + y + GraphService._SVG_COORDINATE_SEPARATOR + x + GraphService._SVG_COORDINATE_SEPARATOR + (y + curve) + GraphService._SVG_VERTICAL_COMMAND + child.top;
  }

  /** @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} parent @param {import('/models/GraphLayoutBox.ts').GraphLayoutBox} child @returns {number} */
  #branchStart(parent, child) {
    const x = child.left + child.width / GraphService._MIDPOINT_DIVISOR;
    const distance = parent.left + parent.width / GraphService._MIDPOINT_DIVISOR - x;
    return x + Math.sign(distance) * Math.min(GraphService._CURVE, Math.abs(distance));
  }

  /** @param {string | undefined} identifier @returns {number} */
  #zoom(identifier) {
    const key = this.#identifier(identifier);
    return this.#storedZoom(this.#zooms.get(key));
  }

  /** @param {number | undefined} zoom @returns {number} */
  #storedZoom(zoom) {
    return zoom ?? 1;
  }

  /** @param {HTMLElement} group @param {string} action @returns {void} */
  #changeZoom(group, action) {
    const identifier = this.#identifier(group.dataset.group);
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
