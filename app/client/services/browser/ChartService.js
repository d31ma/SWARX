@Service
export default class ChartService {
  /** @type {number} */
  static _CPU_MAXIMUM = 100;
  /** @type {number} */
  static _TICK_LIMIT = 5;
  /** @type {number} */
  static _FONT_SIZE = 12;
  /** @type {number} */
  static _MILLISECONDS = 1000;
  /** @type {Intl.DateTimeFormatOptions['hour']} */
  static _TWO_DIGIT_TIME = '2-digit';
  /** @type {number} */
  static _MINIMUM_POINTS = 4;
  /** @type {string} */
  static _BLUE = '#8bbaff';
  /** @type {string} */
  static _VIOLET = '#ae9cff';
  /** @type {string} */
  static _CPU_MODE = 'cpu';
  /** @type {string} */
  static _NETWORK_MODE = 'network';
  /** @type {string} */
  static _DISK_MODE = 'disk';
  /** @type {string} */
  static _GRID_COLOR = '#303644';
  /** @type {string} */
  static _TIME_COLOR = '#a1acbf';
  /** @type {string} */
  static _VALUE_COLOR = '#bac4d7';
  /** @type {number} */
  static _SERIES_BORDER_WIDTH = 2;
  /** @type {number} */
  static _VALUE_DECIMALS = 2;
  /** @type {number} */
  static _HOVER_RADIUS = 4;
  /** @type {number} */
  static _HIT_RADIUS = 12;
  /** @type {number} */
  static _TOP_PADDING = 4;
  /** @type {Map<string, string>} */
  static _COLORS = new Map([
    [ChartService._CPU_MODE, ChartService._BLUE], ['resident', ChartService._VIOLET],
    ['received', ChartService._BLUE], ['sent', '#e7bc7b'],
    ['disk-read', ChartService._BLUE], ['disk-write', ChartService._VIOLET]
  ]);
  /** @type {Map<string, import('/models/ChartInstance.ts').ChartInstance>} */
  #charts = new Map();

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  draw(view, selection) {
    this.#drawAll(view, selection);
  }

  /** @returns {void} */
  @OnStop
  dispose() {
    this.#dispose();
  }

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  #drawAll(view, selection) {
    const groups = this.#groups(selection);
    this.#removeMissing(groups);
    this.#drawCanvases(view, groups, selection.mode);
  }

  /** @param {HTMLElement} view @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string} mode @returns {void} */
  #drawCanvases(view, groups, mode) {
    for (const canvas of view.querySelectorAll('canvas.resource-chart')) {
      this.#drawCanvas(canvas, groups, mode);
    }
  }

  /** @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {import('/models/RuntimeView.ts').RuntimeView[]} */
  #groups(selection) {
    return selection.model?.chartResources ?? [];
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @returns {void} */
  #removeMissing(groups) {
    const identifiers = new Set(groups.map(
      /** @param {import('/models/RuntimeView.ts').RuntimeView} group @returns {string} */
      group => group.id
    ));
    for (const [identifier, chart] of this.#charts) {
      this.#reconcileChart(identifier, chart, identifiers);
    }
  }

  /** @param {string} identifier @param {import('/models/ChartInstance.ts').ChartInstance} chart @param {Set<string>} identifiers @returns {void} */
  #reconcileChart(identifier, chart, identifiers) {
    const absent = !identifiers.has(identifier);
    this.#removeAbsent(identifier, chart, absent);
  }

  /** @param {string} identifier @param {import('/models/ChartInstance.ts').ChartInstance} chart @param {boolean} absent @returns {void} */
  #removeAbsent(identifier, chart, absent) {
    if (!absent) {
      return;
    }
    chart.destroy();
    this.#charts.delete(identifier);
  }

  /** @param {Element} element @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string} mode @returns {void} */
  #drawCanvas(element, groups, mode) {
    const canvas = /** @type {HTMLCanvasElement} */ (element);
    const found = groups.find(
      /** @param {import('/models/RuntimeView.ts').RuntimeView} item @returns {boolean} */
      item => item.id === canvas.dataset.group
    );
    const group = this.#canvasGroup(found);
    const constructor = this.#chartConstructor();
    this.#validate(/** @type {HTMLElement} */ (canvas.closest('.history-card')), {constructor, canvas, group, mode});
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView | undefined} group @returns {import('/models/RuntimeView.ts').RuntimeView | null} */
  #canvasGroup(group) {
    return group ?? null;
  }

  /** @returns {import('/models/ChartConstructor.ts').ChartConstructor | undefined} */
  #chartConstructor() {
    return /** @type {Window & {Chart?: import('/models/ChartConstructor.ts').ChartConstructor}} */ (window).Chart;
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: Element | null, group: import('/models/RuntimeView.ts').RuntimeView | null, mode: string}} input @returns {void} */
  #validate(view, input) {
    if (![input.constructor, input.canvas, input.group].every(Boolean)) {
      return;
    }
    this.#render(view, input);
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: Element | null, group: import('/models/RuntimeView.ts').RuntimeView | null, mode: string}} input @returns {void} */
  #render(view, input) {
    const canvas = /** @type {HTMLCanvasElement} */ (input.canvas);
    const group = /** @type {import('/models/RuntimeView.ts').RuntimeView} */ (input.group);
    const projection = this.#projection(group, input.mode);
    this.#renderProjection(view, {...input, group, canvas}, projection);
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView} group @param {string} mode @returns {import('/models/ChartView.ts').ChartView | undefined} */
  #projection(group, mode) {
    const found = group.charts.find(
      /** @param {import('/models/ChartView.ts').ChartView} view @returns {boolean} */
      view => view.mode === mode
    );
    return this.#firstProjection(found, group.charts);
  }

  /** @param {import('/models/ChartView.ts').ChartView | undefined} found @param {import('/models/ChartView.ts').ChartView[]} charts @returns {import('/models/ChartView.ts').ChartView | undefined} */
  #firstProjection(found, charts) {
    return found ?? charts[0];
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: HTMLCanvasElement, group: import('/models/RuntimeView.ts').RuntimeView, mode: string}} input @param {import('/models/ChartView.ts').ChartView | undefined} projection @returns {void} */
  #renderProjection(view, input, projection) {
    if (projection === undefined) {
      return;
    }
    this.#applyProjection(view, input, projection);
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: HTMLCanvasElement, group: import('/models/RuntimeView.ts').RuntimeView, mode: string}} input @param {import('/models/ChartView.ts').ChartView} projection @returns {void} */
  #applyProjection(view, input, projection) {
    const datasets = this.#datasets(projection);
    const options = this.#options(projection);
    this.#update(input.canvas, /** @type {import('/models/ChartConstructor.ts').ChartConstructor} */ (input.constructor), {datasets, options});
    this.#count(input.canvas.dataset.group, projection.measuredCount);
    this.#empty(view, projection);
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {import('/models/ChartDataset.ts').ChartDataset[]} */
  #datasets(projection) {
    /** @type {import('/models/ChartDataset.ts').ChartDataset[]} */
    const datasets = [];
    for (const series of projection.series) {
      datasets.push(this.#dataset(series, projection));
    }
    return datasets;
  }

  /** @param {string} identifier @returns {string} */
  #color(identifier) {
    return ChartService._COLORS.get(identifier) ?? ChartService._BLUE;
  }

  /** @param {import('/models/ChartSeries.ts').ChartSeries} series @param {import('/models/ChartView.ts').ChartView} projection @returns {import('/models/ChartDataset.ts').ChartDataset} */
  #dataset(series, projection) {
    const color = this.#color(series.id);
    return {
      label: series.label,
      data: series.points,
      borderColor: color,
      backgroundColor: color + '12',
      borderWidth: ChartService._SERIES_BORDER_WIDTH,
      pointRadius: this.#pointRadius(projection.sampleCount),
      pointHoverRadius: ChartService._HOVER_RADIUS,
      pointHitRadius: ChartService._HIT_RADIUS,
      spanGaps: false,
      tension: 0,
      fill: projection.mode !== ChartService._NETWORK_MODE
    };
  }

  /** @param {number} count @returns {number} */
  #pointRadius(count) {
    if (count < ChartService._MINIMUM_POINTS) {
      return 3;
    }
    return 0;
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {Record<string, unknown>} */
  #options(projection) {
    const mode = projection.mode;
    const unit = projection.unit;
    return {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      normalized: false,
      interaction: {mode: 'nearest', axis: 'x', intersect: false},
      layout: {padding: {top: ChartService._TOP_PADDING, right: 5}},
      plugins: {
        legend: {
          display: [ChartService._NETWORK_MODE, ChartService._DISK_MODE].includes(mode),
          labels: {color: '#ccd2e0', font: {size: ChartService._FONT_SIZE}, boxWidth: 14}
        },
        tooltip: {
          enabled: true,
          backgroundColor: '#242938',
          titleColor: '#f1f4ff',
          bodyColor: '#e2e7f4',
          borderColor: '#586080',
          borderWidth: 1,
          padding: 11,
          callbacks: {
            title: this.#tooltipTime,
            label: /** @param {{dataset: {label: string}, parsed: {y: number}}} item @returns {string} */ item => this.#tooltipValue(item, unit)
          }
        }
      },
      scales: {
        x: this.#timeAxis(projection),
        y: this.#valueAxis(mode, unit)
      }
    };
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {Record<string, unknown>} */
  #timeAxis(projection) {
    return {
      type: 'linear',
      min: projection.windowStart,
      max: projection.windowEnd,
      grid: {color: ChartService._GRID_COLOR},
      ticks: {color: ChartService._TIME_COLOR, maxTicksLimit: ChartService._TICK_LIMIT, font: {size: ChartService._FONT_SIZE}, callback: this.#timeTick},
      title: {display: true, text: 'Local time · last two minutes', color: ChartService._TIME_COLOR, font: {size: ChartService._FONT_SIZE}}
    };
  }

  /** @param {string} mode @param {string | undefined} unit @returns {Record<string, unknown>} */
  #valueAxis(mode, unit) {
    return {
      beginAtZero: mode !== 'memory',
      suggestedMax: this.#maximum(mode),
      grid: {color: ChartService._GRID_COLOR},
      ticks: {color: ChartService._VALUE_COLOR, maxTicksLimit: ChartService._TICK_LIMIT, font: {size: ChartService._FONT_SIZE}},
      title: {display: true, text: unit, color: ChartService._VALUE_COLOR, font: {size: ChartService._FONT_SIZE}}
    };
  }

  /** @param {string} mode @returns {number | undefined} */
  #maximum(mode) {
    if (mode === ChartService._CPU_MODE) {
      return ChartService._CPU_MAXIMUM;
    }
    return undefined;
  }

  /** @param {Array<{parsed: {x: number}}>} items @returns {string} */
  #tooltipTime(items) {
    return new Date(items[0].parsed.x * ChartService._MILLISECONDS).toLocaleTimeString();
  }

  /** @param {{dataset: {label: string}, parsed: {y: number}}} item @param {string | undefined} unit @returns {string} */
  #tooltipValue(item, unit) {
    return item.dataset.label + ': ' + Number(item.parsed.y).toFixed(ChartService._VALUE_DECIMALS) + ' ' + unit;
  }

  /** @param {number | string} value @returns {string} */
  #timeTick(value) {
    const time = new Date(Number(value) * ChartService._MILLISECONDS);
    return time.toLocaleTimeString([], {
      hour: ChartService._TWO_DIGIT_TIME,
      minute: ChartService._TWO_DIGIT_TIME,
      second: ChartService._TWO_DIGIT_TIME
    });
  }

  /** @param {HTMLCanvasElement} canvas @param {import('/models/ChartConstructor.ts').ChartConstructor} constructor @param {{datasets: import('/models/ChartDataset.ts').ChartDataset[], options: Record<string, unknown>}} values @returns {void} */
  #update(canvas, constructor, values) {
    const identifier = this.#identifier(canvas.dataset.group);
    const previous = this.#charts.get(identifier);
    this.#replaceCanvas(canvas, previous?.canvas);
    this.#updateOrCreate(canvas, constructor, {...values, identifier}, previous);
  }

  /** @param {string | undefined} identifier @returns {string} */
  #identifier(identifier) {
    return identifier ?? '';
  }

  /** @param {HTMLCanvasElement} canvas @param {HTMLCanvasElement | undefined} previous @returns {void} */
  #replaceCanvas(canvas, previous) {
    const replace = [previous !== undefined, canvas !== previous].every(Boolean);
    if (!replace) {
      return;
    }
    canvas.parentNode?.replaceChild(/** @type {HTMLCanvasElement} */ (previous), canvas);
  }

  /** @param {HTMLCanvasElement} canvas @param {import('/models/ChartConstructor.ts').ChartConstructor} constructor @param {{datasets: import('/models/ChartDataset.ts').ChartDataset[], options: Record<string, unknown>, identifier: string}} values @param {import('/models/ChartInstance.ts').ChartInstance | undefined} previous @returns {void} */
  #updateOrCreate(canvas, constructor, values, previous) {
    if (!previous) {
      const chart = new constructor(canvas, {type: 'line', data: {datasets: values.datasets}, options: values.options});
      this.#charts.set(values.identifier, chart);
      return;
    }
    previous.data = {datasets: values.datasets};
    previous.options = values.options;
    previous.update('none');
  }

  /** @param {string | undefined} identifier @param {number} points @returns {void} */
  #count(identifier, points) {
    this.#setCount(this.#identifier(identifier), points);
  }

  /** @param {string} identifier @param {number} points @returns {void} */
  #setCount(identifier, points) {
    const chart = this.#charts.get(identifier);
    if (!chart) {
      return;
    }
    chart.canvas.dataset.points = String(points);
  }

  /** @param {HTMLElement} view @param {import('/models/ChartView.ts').ChartView} projection @returns {void} */
  #empty(view, projection) {
    const empty = /** @type {HTMLElement | null} */ (view.querySelector('.chart-empty'));
    this.#showEmpty(empty, projection.empty, projection.emptyText);
  }

  /** @param {HTMLElement | null} empty @param {boolean} noData @param {string} message @returns {void} */
  #showEmpty(empty, noData, message) {
    const text = this.#emptyElement(empty);
    this.#replaceText(text, message);
    this.#hideEmpty(text, noData);
  }

  /** @param {HTMLElement | null} element @param {string} text @returns {void} */
  #replaceText(element, text) {
    const change = [element !== null, element?.textContent !== text].every(Boolean);
    if (!change) {
      return;
    }
    /** @type {HTMLElement} */ (element).textContent = text;
  }

  /** @param {HTMLElement | null} element @returns {HTMLElement | null} */
  #emptyElement(element) {
    return element;
  }

  /** @param {HTMLElement | null} element @param {boolean} noData @returns {void} */
  #hideEmpty(element, noData) {
    if (!element) {
      return;
    }
    element.hidden = !noData;
  }

  /** @returns {void} */
  #dispose() {
    for (const chart of this.#charts.values()) {
      chart.destroy();
    }
    this.#charts.clear();
  }
}
