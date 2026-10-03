import { DomUtils } from '../../utils/DomUtils.js';

export class GraphGroupRenderer {
  /** @type {number} */
  static #PADDING = 20;
  /** @type {number} */
  static #CARD_WIDTH = 280;
  /** @type {number} */
  static #CARD_HEIGHT = 212;
  /** @type {number} */
  static #LEVEL_SPACING = 296;
  /** @type {number} */
  static #SIBLING_GAP = 40;
  /** @type {number} */
  static #TRUNK_LANE = 28;
  /** @type {number} */
  static #FIT_STEPS = 12;
  /** @type {number} */
  static #FIT_MINIMUM = 0.001;
  /** @type {number} */
  static #CURVE_SIZE = 12;
  /** @type {number} */
  static #PERCENT = 100;
  /** @type {number} */
  static #EMPTY_COUNT = 0;
  /** @type {number} */
  static #SINGLE_COUNT = 1;
  /** @type {number} */
  static #CENTER_DIVISOR = 2;
  /** @type {string} */
  static #PIXEL_UNIT = 'px';
  /** @type {string} */
  static #LABEL_ATTRIBUTE = 'aria-label';
  /** @type {string} */
  static #ROLE_ATTRIBUTE = 'role';
  /** @type {string} */
  static #BUTTON_TAG = 'button';
  /** @type {string} */
  static #DIV_TAG = 'div';
  /** @type {string} */
  static #SPAN_TAG = 'span';
  /** @type {string} */
  static #MEASUREMENT_CLASS = 'graph-measurement';
  /** @type {string} */
  static #RESIDENT_LABEL = ' · RSS ';
  /** @type {string} */
  static #EMPTY_TEXT = '';
  /** @type {string} */
  static #MOVE_PREFIX = 'M ';
  /** @type {string} */
  static #LINE_PREFIX = ' L ';
  /** @type {string} */
  static #COORDINATE_SEPARATOR = ' ';
  /** @type {import('/models/RuntimeView.ts').RuntimeView} */
  #group;
  /** @type {import('/models/ViewSelection.ts').ViewSelection} */
  #selection;
  /** @type {number} */
  #zoom;
  /** @type {number} */
  #viewportWidth;
  /** @type {Map<string, number>} */
  #heights = new Map();
  /** @type {Map<string, number>} */
  #tops = new Map();
  /** @type {Map<string, Array<{children: string[], width: number, height: number}>>} */
  #rows = new Map();
  /** @type {Map<string, number>} */
  #lanes = new Map();
  /** @type {HTMLElement} */
  #canvas = DomUtils.element(GraphGroupRenderer.#DIV_TAG, 'graph-canvas');
  /** @type {HTMLElement} */
  #sized = DomUtils.element(GraphGroupRenderer.#DIV_TAG, 'group-graph-size');
  /** @type {SVGElement} */
  #edges = DomUtils.svg(this.#canvas, 'svg', 'graph-edges');
  /** @type {Map<string, {x: number, y: number}>} */
  #positions = new Map();
  /** @type {Map<string, import('/models/ProcessGraphNode.ts').ProcessGraphNode>} */
  #records = new Map();
  /** @type {Map<string, string[]>} */
  #children = new Map();
  /** @type {Map<string, number>} */
  #levels = new Map();
  /** @type {Map<string, number>} */
  #widths = new Map();
  /** @type {Map<string, number>} */
  #lefts = new Map();
  /** @type {string[]} */
  #roots = [];
  /** @type {number} */
  #width = GraphGroupRenderer.#CARD_WIDTH;
  /** @type {number} */
  #height = GraphGroupRenderer.#CARD_HEIGHT;

  /** @param {import('/models/RuntimeView.ts').RuntimeView} group @param {import('/models/ViewSelection.ts').ViewSelection} selection @param {number} zoom @param {number} viewportWidth */
  constructor(group, selection, zoom, viewportWidth) {
    this.#group = group;
    this.#selection = selection;
    this.#zoom = zoom;
    this.#viewportWidth = viewportWidth;
  }

  /** @returns {{width: number, height: number}} */
  measure() {
    this.#layout();
    return {width: this.#width, height: this.#height};
  }

  /** @param {number} width @param {number} height @returns {number} */
  fit(width, height) {
    return this.#fitBounds(width, height).minimum;
  }

  /** @param {number} width @param {number} height @returns {{minimum: number, maximum: number}} */
  #fitBounds(width, height) {
    const initial = {minimum: GraphGroupRenderer.#FIT_MINIMUM, maximum: 1};
    let bounds = initial;
    for (let step = 0; step < GraphGroupRenderer.#FIT_STEPS; step += 1) {
      bounds = this.#fitStep(bounds, width, height);
    }
    return bounds;
  }

  /** @param {{minimum: number, maximum: number}} bounds @param {number} width @param {number} height @returns {{minimum: number, maximum: number}} */
  #fitStep(bounds, width, height) {
    const candidate = (bounds.minimum + bounds.maximum) / GraphGroupRenderer.#CENTER_DIVISOR;
    const measured = new GraphGroupRenderer(this.#group, this.#selection, candidate, width).measure();
    const fits = [measured.width * candidate <= width, measured.height * candidate <= height].every(Boolean);
    if (fits) {
      return {minimum: candidate, maximum: bounds.maximum};
    }
    return {minimum: bounds.minimum, maximum: candidate};
  }

  /** @returns {HTMLElement} */
  render() {
    const box = this.#box();
    box.id = 'graph-panel-' + this.#group.pid;
    box.setAttribute(GraphGroupRenderer.#ROLE_ATTRIBUTE, 'tabpanel');
    box.setAttribute('aria-labelledby', 'graph-tab-' + this.#group.pid);
    const scroll = this.#scroll();
    box.append(this.#controls());
    this.#edges.setAttribute('aria-hidden', 'true');
    this.#sized.append(this.#canvas);
    scroll.append(this.#sized);
    box.append(scroll);
    this.#layout();
    this.#nodes();
    this.#connections();
    this.#size();
    return box;
  }

  /** @returns {HTMLElement} */
  #controls() {
    const tools = DomUtils.element(GraphGroupRenderer.#DIV_TAG, 'graph-group-tools');
    tools.setAttribute(GraphGroupRenderer.#ROLE_ATTRIBUTE, 'group');
    tools.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, 'Zoom process tree for ' + this.#group.name);
    tools.append(this.#zoomButton('out', '−', 'Zoom out process tree for '));
    const value = DomUtils.label(tools, GraphGroupRenderer.#SPAN_TAG, Math.round(this.#zoom * GraphGroupRenderer.#PERCENT) + '%', 'graph-zoom-value');
    value.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, 'Current zoom for ' + this.#group.name);
    tools.append(this.#zoomButton('in', '+', 'Zoom in process tree for '));
    tools.append(this.#zoomButton('fit', 'Fit', 'Fit process tree for '));
    return tools;
  }

  /** @param {string} action @param {string} text @param {string} label @returns {HTMLElement} */
  #zoomButton(action, text, label) {
    const button = DomUtils.element(GraphGroupRenderer.#BUTTON_TAG, 'graph-zoom-button');
    button.setAttribute('type', GraphGroupRenderer.#BUTTON_TAG);
    button.textContent = text;
    button.dataset.zoom = action;
    button.dataset.group = this.#group.id;
    button.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, label + this.#group.name);
    return button;
  }

  /** @returns {HTMLElement} */
  #box() {
    const box = DomUtils.element('section', 'graph-group');
    box.dataset.group = this.#group.id;
    box.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, this.#group.name);
    DomUtils.label(box, 'h3', this.#group.name, 'graph-group-title');
    const text = this.#group.family + ' · ' + this.#group.scopeLabel + ' · group CPU ' + this.#group.cpu + GraphGroupRenderer.#RESIDENT_LABEL + this.#group.memory;
    DomUtils.label(box, 'p', text, 'graph-group-meta');
    return box;
  }

  /** @returns {HTMLElement} */
  #scroll() {
    const scroll = DomUtils.element(GraphGroupRenderer.#DIV_TAG, 'group-graph-scroll');
    scroll.dataset.group = this.#group.id;
    scroll.tabIndex = 0;
    scroll.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, 'Process tree for ' + this.#group.name);
    scroll.setAttribute(GraphGroupRenderer.#ROLE_ATTRIBUTE, 'region');
    return scroll;
  }

  /** @returns {void} */
  #layout() {
    this.#index();
    this.#roots = this.#group.graph.roots.slice();
    this.#measureWidths();
    this.#allocateRoots();
    this.#placeNodes();
    this.#bounds();
  }

  /** @returns {void} */
  #index() {
    for (const process of this.#group.graph.nodes) {
      this.#indexNode(process);
    }
  }

  /** @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode} process @returns {void} */
  #indexNode(process) {
    this.#records.set(process.identity, process);
    this.#children.set(process.identity, process.childIdentities.slice());
    this.#levels.set(process.identity, process.graphDepth);
  }

  /** @returns {string[]} */
  #bottomUp() {
    return Array.from(this.#records.keys()).sort(
      /** @param {string} left @param {string} right @returns {number} */
      (left, right) => /** @type {number} */ (this.#levels.get(right)) - /** @type {number} */ (this.#levels.get(left))
    );
  }

  /** @returns {void} */
  #measureWidths() {
    for (const identifier of this.#bottomUp()) {
      this.#measureSubtree(identifier);
    }
  }

  /** @param {string} identifier @returns {void} */
  #measureSubtree(identifier) {
    const children = /** @type {string[]} */ (this.#children.get(identifier));
    const level = /** @type {number} */ (this.#levels.get(identifier));
    const rows = this.#partitionRows(children, this.#levelBudget(level));
    const widths = this.#rowWidths(rows);
    const width = Math.max(GraphGroupRenderer.#CARD_WIDTH, Math.max(0, ...widths) + Number(children.length > GraphGroupRenderer.#SINGLE_COUNT) * GraphGroupRenderer.#TRUNK_LANE * GraphGroupRenderer.#CENTER_DIVISOR);
    const height = GraphGroupRenderer.#CARD_HEIGHT + this.#rowsHeight(rows) + Number(rows.length > GraphGroupRenderer.#EMPTY_COUNT) * this.#rowGap();
    this.#rows.set(identifier, rows);
    this.#widths.set(identifier, width);
    this.#heights.set(identifier, height);
  }

  /** @param {number} level @returns {number} */
  #levelBudget(level) {
    return Math.max(GraphGroupRenderer.#CARD_WIDTH, this.#viewportWidth / this.#zoom - GraphGroupRenderer.#PADDING * GraphGroupRenderer.#CENTER_DIVISOR - (level + 1) * GraphGroupRenderer.#TRUNK_LANE * GraphGroupRenderer.#CENTER_DIVISOR);
  }

  /** @returns {number} */
  #rowGap() {
    return GraphGroupRenderer.#LEVEL_SPACING - GraphGroupRenderer.#CARD_HEIGHT;
  }

  /** @param {string[]} children @param {number} budget @returns {Array<{children: string[], width: number, height: number}>} */
  #partitionRows(children, budget) {
    const rows = this.#packedRows(children, budget);
    return this.#filledRows(rows);
  }

  /** @param {string[]} children @param {number} budget @returns {Array<{children: string[], width: number, height: number}>} */
  #packedRows(children, budget) {
    /** @type {Array<{children: string[], width: number, height: number}>} */
    const rows = [{children: [], width: 0, height: 0}];
    for (const identifier of children) {
      this.#packChild(rows, identifier, budget);
    }
    return rows;
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @returns {Array<{children: string[], width: number, height: number}>} */
  #filledRows(rows) {
    return rows.filter(/** @param {{children: string[], width: number, height: number}} row @returns {boolean} */ row => row.children.length > GraphGroupRenderer.#EMPTY_COUNT);
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @param {string} identifier @param {number} budget @returns {Array<{children: string[], width: number, height: number}>} */
  #packChild(rows, identifier, budget) {
    const row = rows[rows.length - 1];
    const width = /** @type {number} */ (this.#widths.get(identifier));
    const needed = row.width + width + Number(row.children.length > GraphGroupRenderer.#EMPTY_COUNT) * GraphGroupRenderer.#SIBLING_GAP;
    const fits = [needed <= budget, row.children.length === GraphGroupRenderer.#EMPTY_COUNT].some(Boolean);
    if (!fits) {
      rows.push({children: [], width: 0, height: 0});
    }
    this.#appendMeasured(rows[rows.length - 1], identifier, width);
    return rows;
  }

  /** @param {{children: string[], width: number, height: number}} row @param {string} identifier @param {number} width @returns {void} */
  #appendMeasured(row, identifier, width) {
    row.width += width + Number(row.children.length > GraphGroupRenderer.#EMPTY_COUNT) * GraphGroupRenderer.#SIBLING_GAP;
    row.height = Math.max(row.height, /** @type {number} */ (this.#heights.get(identifier)));
    row.children.push(identifier);
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @returns {number} */
  #rowsHeight(rows) {
    return this.#totalRowHeight(rows) + Math.max(0, rows.length - 1) * this.#rowGap();
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @returns {number} */
  #totalRowHeight(rows) {
    let total = 0;
    for (const row of rows) {
      total += row.height;
    }
    return total;
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @returns {number[]} */
  #rowWidths(rows) {
    return rows.map(/** @param {{children: string[], width: number, height: number}} row @returns {number} */ row => row.width);
  }

  /** @returns {void} */
  #allocateRoots() {
    const rows = this.#partitionRows(this.#roots, this.#levelBudget(-1));
    const width = Math.max(GraphGroupRenderer.#CARD_WIDTH, ...this.#rowWidths(rows));
    const stack = this.#allocateRows(rows, GraphGroupRenderer.#PADDING, GraphGroupRenderer.#PADDING, width);
    this.#width = width + GraphGroupRenderer.#PADDING * GraphGroupRenderer.#CENTER_DIVISOR;
    this.#height = this.#rowsHeight(rows) + GraphGroupRenderer.#PADDING * GraphGroupRenderer.#CENTER_DIVISOR;
    this.#walkAllocation(stack);
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @param {number} left @param {number} top @param {number} width @returns {Array<{identifier: string, left: number, top: number}>} */
  #allocateRows(rows, left, top, width) {
    /** @type {Array<{identifier: string, left: number, top: number}>} */
    const stack = [];
    let y = top;
    for (const row of rows) {
      stack.push(...this.#allocateRow(row, left + (width - row.width) / GraphGroupRenderer.#CENTER_DIVISOR, y));
      y += row.height + this.#rowGap();
    }
    return stack;
  }

  /** @param {{children: string[], width: number, height: number}} row @param {number} left @param {number} top @returns {Array<{identifier: string, left: number, top: number}>} */
  #allocateRow(row, left, top) {
    /** @type {Array<{identifier: string, left: number, top: number}>} */
    const entries = [];
    let x = left;
    for (const identifier of row.children) {
      entries.push({identifier, left: x, top});
      x += /** @type {number} */ (this.#widths.get(identifier)) + GraphGroupRenderer.#SIBLING_GAP;
    }
    return entries;
  }

  /** @param {Array<{identifier: string, left: number, top: number}>} stack @returns {void} */
  #walkAllocation(stack) {
    while (stack.length > GraphGroupRenderer.#EMPTY_COUNT) {
      this.#allocationEntry(/** @type {{identifier: string, left: number, top: number}} */ (stack.pop()), stack);
    }
  }

  /** @param {{identifier: string, left: number, top: number}} entry @param {Array<{identifier: string, left: number, top: number}>} stack @returns {void} */
  #allocationEntry(entry, stack) {
    this.#lefts.set(entry.identifier, entry.left);
    this.#tops.set(entry.identifier, entry.top);
    const rows = /** @type {Array<{children: string[], width: number, height: number}>} */ (this.#rows.get(entry.identifier));
    const width = /** @type {number} */ (this.#widths.get(entry.identifier));
    stack.push(...this.#allocateRows(rows, entry.left, entry.top + GraphGroupRenderer.#LEVEL_SPACING, width));
  }

  /** @returns {void} */
  #placeNodes() {
    for (const identifier of this.#records.keys()) {
      this.#place(identifier);
    }
  }

  /** @param {string} identifier @returns {void} */
  #place(identifier) {
    const width = /** @type {number} */ (this.#widths.get(identifier));
    const x = /** @type {number} */ (this.#lefts.get(identifier)) + (width - GraphGroupRenderer.#CARD_WIDTH) / GraphGroupRenderer.#CENTER_DIVISOR;
    const y = /** @type {number} */ (this.#tops.get(identifier));
    this.#positions.set(identifier, {x, y});
  }

  /** @returns {void} */
  #bounds() {
    this.#height = Math.max(GraphGroupRenderer.#CARD_HEIGHT, this.#height);
  }

  /** @returns {void} */
  #nodes() {
    for (const process of this.#records.values()) {
      this.#node(process);
    }
  }

  /** @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode} process @returns {void} */
  #node(process) {
    const position = /** @type {{x: number, y: number}} */ (this.#positions.get(process.identity));
    const node = DomUtils.element(GraphGroupRenderer.#BUTTON_TAG, 'graph-node process-pick' + this.#selected(process.pid));
    node.dataset.pid = String(process.pid);
    node.dataset.group = this.#group.id;
    node.dataset.depth = String(process.graphDepth);
    node.dataset.nativeDepth = String(process.depth);
    node.dataset.identity = process.identity;
    node.dataset.edgeStatus = process.edgeStatus;
    node.setAttribute('title', process.edgeLabel);
    node.setAttribute('aria-description', process.edgeLabel);
    node.style.left = position.x + GraphGroupRenderer.#PIXEL_UNIT;
    node.style.top = position.y + GraphGroupRenderer.#PIXEL_UNIT;
    node.style.width = GraphGroupRenderer.#CARD_WIDTH + GraphGroupRenderer.#PIXEL_UNIT;
    node.style.minHeight = GraphGroupRenderer.#CARD_HEIGHT + GraphGroupRenderer.#PIXEL_UNIT;
    node.setAttribute(GraphGroupRenderer.#LABEL_ATTRIBUTE, process.name + ' PID ' + process.pid);
    this.#labels(node, process);
    this.#canvas.append(node);
  }

  /** @param {number} identifier @returns {string} */
  #selected(identifier) {
    const model = this.#selection.model;
    const process = this.#selectedProcess();
    const selected = model?.selected === this.#group.id;
    return this.#selectionClass(selected, identifier === process);
  }

  /** @returns {number} */
  #selectedProcess() {
    return this.#selection.pid || this.#group.pid;
  }

  /** @param {boolean} group @param {boolean} process @returns {string} */
  #selectionClass(group, process) {
    if ([group, process].every(Boolean)) {
      return ' selected';
    }
    return GraphGroupRenderer.#EMPTY_TEXT;
  }

  /** @param {HTMLElement} node @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode} process @returns {void} */
  #labels(node, process) {
    DomUtils.label(node, 'strong', process.name, 'graph-name');
    DomUtils.label(node, GraphGroupRenderer.#SPAN_TAG, 'PID ' + process.pid + ' · PPID ' + process.parent, 'graph-identity');
    DomUtils.label(node, GraphGroupRenderer.#SPAN_TAG, 'CPU ' + process.cpu + GraphGroupRenderer.#RESIDENT_LABEL + process.memory, GraphGroupRenderer.#MEASUREMENT_CLASS);
    this.#optionalLabel(node, process.networkLabel);
    this.#optionalLabel(node, process.diskLabel);
    this.#ownerLabel(node, process.owner);
  }


  /** @param {HTMLElement} node @param {import('/models/ProcessOwner.ts').ProcessOwner | null | undefined} owner @returns {void} */
  #ownerLabel(node, owner) {
    if (!owner) {
      return;
    }
    DomUtils.label(node, GraphGroupRenderer.#SPAN_TAG, 'Reported owner · ' + owner.agent_name, 'process-owner');
    DomUtils.label(node, GraphGroupRenderer.#SPAN_TAG, 'Session · ' + owner.session_name + ' · ' + owner.scope_label, 'process-owner');
  }

  /** @param {HTMLElement} node @param {string} text @returns {void} */
  #optionalLabel(node, text) {
    if (text === GraphGroupRenderer.#EMPTY_TEXT) {
      return;
    }
    DomUtils.label(node, GraphGroupRenderer.#SPAN_TAG, text, GraphGroupRenderer.#MEASUREMENT_CLASS);
  }

  /** @returns {void} */
  #connections() {
    const parents = this.#connectionParents();
    this.#connectParents(parents);
  }

  /** @returns {import('/models/ProcessGraphNode.ts').ProcessGraphNode[]} */
  #connectionParents() {
    return Array.from(this.#records.values()).filter(/** @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode} process @returns {boolean} */ process => /** @type {string[]} */ (this.#children.get(process.identity)).length > GraphGroupRenderer.#EMPTY_COUNT);
  }

  /** @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode[]} parents @returns {void} */
  #connectParents(parents) {
    for (const process of parents) {
      this.#parentConnections(process);
    }
  }

  /** @param {import('/models/ProcessGraphNode.ts').ProcessGraphNode} process @returns {void} */
  #parentConnections(process) {
    const parent = /** @type {{x: number, y: number}} */ (this.#positions.get(process.identity));
    const children = /** @type {string[]} */ (this.#children.get(process.identity));
    const input = {identifier: process.identity, parent, children};
    this.#sharedConnection(input);
    this.#branches(input);
  }

  /** @param {{identifier: string, parent: {x: number, y: number}, children: string[]}} input @returns {void} */
  #sharedConnection(input) {
    const rows = /** @type {Array<{children: string[], width: number, height: number}>} */ (this.#rows.get(input.identifier));
    const first = /** @type {{x: number, y: number}} */ (this.#positions.get(rows[0].children[0]));
    const last = /** @type {{x: number, y: number}} */ (this.#positions.get(rows[rows.length - 1].children[0]));
    const center = input.parent.x + GraphGroupRenderer.#CARD_WIDTH / GraphGroupRenderer.#CENTER_DIVISOR;
    const start = input.parent.y + GraphGroupRenderer.#CARD_HEIGHT;
    const firstBus = first.y - this.#rowGap() / GraphGroupRenderer.#CENTER_DIVISOR;
    const lastBus = last.y - this.#rowGap() / GraphGroupRenderer.#CENTER_DIVISOR;
    const left = /** @type {number} */ (this.#lefts.get(input.identifier)) + GraphGroupRenderer.#TRUNK_LANE / GraphGroupRenderer.#CENTER_DIVISOR;
    const wrapped = rows.length > GraphGroupRenderer.#SINGLE_COUNT;
    const lane = this.#trunkLane(wrapped, center, left);
    const path = this.#trunkPath(wrapped, {center, start, firstBus, lastBus, lane});
    this.#lanes.set(input.identifier, lane);
    this.#edge(path, 'shared-trunk', input.identifier);
    this.#rowConnections(rows, {identifier: input.identifier, center, lane, wrapped});
  }

  /** @param {boolean} wrapped @param {number} center @param {number} left @returns {number} */
  #trunkLane(wrapped, center, left) {
    if (!wrapped) {
      return center;
    }
    return left;
  }

  /** @param {boolean} wrapped @param {{center: number, start: number, firstBus: number, lastBus: number, lane: number}} points @returns {string} */
  #trunkPath(wrapped, points) {
    const {center, start, firstBus, lastBus, lane} = points;
    if (!wrapped) {
      return `M ${center} ${start} L ${center} ${firstBus}`;
    }
    const curve = GraphGroupRenderer.#CURVE_SIZE;
    return `M ${center} ${start} L ${center} ${firstBus - curve} Q ${center} ${firstBus} ${center - curve} ${firstBus} L ${lane + curve} ${firstBus} Q ${lane} ${firstBus} ${lane} ${firstBus + curve} L ${lane} ${lastBus}`;
  }

  /** @param {Array<{children: string[], width: number, height: number}>} rows @param {{identifier: string, center: number, lane: number, wrapped: boolean}} input @returns {void} */
  #rowConnections(rows, input) {
    for (let index = 0; index < rows.length; index += 1) {
      this.#rowConnection(rows[index], input, index);
    }
  }

  /** @param {{children: string[], width: number, height: number}} row @param {{identifier: string, center: number, lane: number, wrapped: boolean}} input @param {number} index @returns {void} */
  #rowConnection(row, input, index) {
    const first = /** @type {{x: number, y: number}} */ (this.#positions.get(row.children[0]));
    const starts = row.children.map(/** @param {string} identifier @returns {number} */ identifier => this.#branchStart(identifier, input.lane));
    const junction = first.y - this.#rowGap() / GraphGroupRenderer.#CENTER_DIVISOR;
    const firstStart = this.#firstBusStart(input.wrapped, input.center, starts);
    const left = this.#busStart(index, firstStart, input.lane);
    const right = Math.max(left, ...starts);
    this.#edge(`M ${left} ${junction} L ${right} ${junction}`, 'shared-bus', input.identifier);
  }

  /** @param {string} identifier @param {number} lane @returns {number} */
  #branchStart(identifier, lane) {
    const child = /** @type {{x: number, y: number}} */ (this.#positions.get(identifier));
    const center = child.x + GraphGroupRenderer.#CARD_WIDTH / GraphGroupRenderer.#CENTER_DIVISOR;
    return center - Math.sign(center - lane) * GraphGroupRenderer.#CURVE_SIZE;
  }

  /** @param {boolean} wrapped @param {number} center @param {number[]} starts @returns {number} */
  #firstBusStart(wrapped, center, starts) {
    if (wrapped) {
      return center;
    }
    return Math.min(center, ...starts);
  }

  /** @param {number} row @param {number} center @param {number} lane @returns {number} */
  #busStart(row, center, lane) {
    if (row === GraphGroupRenderer.#EMPTY_COUNT) {
      return center;
    }
    return lane;
  }

  /** @param {string} path @param {string} className @param {string} parent @returns {SVGElement} */
  #edge(path, className, parent) {
    const edge = DomUtils.svg(this.#edges, 'path', className);
    edge.setAttribute('d', path);
    const record = /** @type {import('/models/ProcessGraphNode.ts').ProcessGraphNode} */ (this.#records.get(parent));
    edge.setAttribute('data-parent', String(record.pid));
    edge.setAttribute('data-parent-identity', parent);
    return edge;
  }

  /** @param {{identifier: string, parent: {x: number, y: number}, children: string[]}} input @returns {void} */
  #branches(input) {
    for (const identifier of input.children) {
      this.#branch(input.identifier, input.parent, identifier);
    }
  }

  /** @param {string} parentIdentifier @param {{x: number, y: number}} parent @param {string} identifier @returns {void} */
  #branch(parentIdentifier, parent, identifier) {
    const child = /** @type {{x: number, y: number}} */ (this.#positions.get(identifier));
    const childCenter = child.x + GraphGroupRenderer.#CARD_WIDTH / GraphGroupRenderer.#CENTER_DIVISOR;
    const junction = child.y - this.#rowGap() / GraphGroupRenderer.#CENTER_DIVISOR;
    const lane = /** @type {number} */ (this.#lanes.get(parentIdentifier));
    const bend = -Math.sign(childCenter - lane) * GraphGroupRenderer.#CURVE_SIZE;
    const path = GraphGroupRenderer.#MOVE_PREFIX + (childCenter + bend) + GraphGroupRenderer.#COORDINATE_SEPARATOR + junction + ' Q ' + childCenter + GraphGroupRenderer.#COORDINATE_SEPARATOR + junction + GraphGroupRenderer.#COORDINATE_SEPARATOR + childCenter + GraphGroupRenderer.#COORDINATE_SEPARATOR + (junction + GraphGroupRenderer.#CURVE_SIZE) + GraphGroupRenderer.#LINE_PREFIX + childCenter + GraphGroupRenderer.#COORDINATE_SEPARATOR + child.y;
    const edge = this.#edge(path, 'child-branch', parentIdentifier);
    const record = /** @type {import('/models/ProcessGraphNode.ts').ProcessGraphNode} */ (this.#records.get(identifier));
    edge.setAttribute('data-child', String(record.pid));
    edge.setAttribute('data-child-identity', identifier);
  }

  /** @returns {void} */
  #size() {
    this.#canvas.dataset.layout = 'adaptive-sibling-rows';
    this.#canvas.style.width = this.#width + GraphGroupRenderer.#PIXEL_UNIT;
    this.#canvas.style.height = this.#height + GraphGroupRenderer.#PIXEL_UNIT;
    this.#canvas.style.transform = 'scale(' + this.#zoom + ')';
    this.#edges.setAttribute('width', String(this.#width));
    this.#edges.setAttribute('height', String(this.#height));
    this.#sized.style.width = this.#width * this.#zoom + GraphGroupRenderer.#PIXEL_UNIT;
    this.#sized.style.height = this.#height * this.#zoom + GraphGroupRenderer.#PIXEL_UNIT;
    const root = this.#rootPosition();
    this.#scrollCenter(root);
  }

  /** @returns {{x: number, y: number}} */
  #rootPosition() {
    const identifier = this.#roots[0];
    return this.#positions.get(identifier) ?? {x: 0, y: 0};
  }

  /** @param {{x: number, y: number}} root @returns {void} */
  #scrollCenter(root) {
    const scroll = /** @type {HTMLElement} */ (this.#sized.parentElement);
    scroll.dataset.rootCenter = String((root.x + GraphGroupRenderer.#CARD_WIDTH / GraphGroupRenderer.#CENTER_DIVISOR) * this.#zoom);
  }
}
