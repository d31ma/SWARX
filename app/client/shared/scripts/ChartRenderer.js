export class ChartRenderer {
  /** @type {number} */
  static #CPU_MAXIMUM = 100;
  /** @type {number} */
  static #TICK_LIMIT = 5;
  /** @type {number} */
  static #FONT_SIZE = 12;
  /** @type {number} */
  static #MILLISECONDS = 1000;
  /** @type {Intl.DateTimeFormatOptions['hour']} */
  static #TWO_DIGIT_TIME = '2-digit';
  /** @type {number} */
  static #MINIMUM_POINTS = 4;
  /** @type {string} */
  static #BLUE = '#8bbaff';
  /** @type {string} */
  static #VIOLET = '#ae9cff';
  /** @type {string} */
  static #CPU_MODE = 'cpu';
  /** @type {string} */
  static #NETWORK_MODE = 'network';
  /** @type {string} */
  static #DISK_MODE = 'disk';
  /** @type {string} */
  static #GRID_COLOR = '#303644';
  /** @type {string} */
  static #TIME_COLOR = '#a1acbf';
  /** @type {string} */
  static #VALUE_COLOR = '#bac4d7';
  /** @type {number} */
  static #SERIES_BORDER_WIDTH = 2;
  /** @type {number} */
  static #VALUE_DECIMALS = 2;
  /** @type {number} */
  static #HOVER_RADIUS = 4;
  /** @type {number} */
  static #HIT_RADIUS = 12;
  /** @type {number} */
  static #TOP_PADDING = 4;
  /** @type {Map<string, string>} */
  static #COLORS = new Map([
    [ChartRenderer.#CPU_MODE, ChartRenderer.#BLUE], ['resident', ChartRenderer.#VIOLET],
    ['received', ChartRenderer.#BLUE], ['sent', '#e7bc7b'],
    ['disk-read', ChartRenderer.#BLUE], ['disk-write', ChartRenderer.#VIOLET]
  ]);
  /** @type {Map<string, import('/models/ChartInstance.ts').ChartInstance>} */
  static #charts = new Map();

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  static draw(view, selection) {
    ChartRenderer.#drawAll(view, selection);
  }

  /** @returns {void} */
  static dispose() {
    ChartRenderer.#dispose();
  }

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  static #drawAll(view, selection) {
    const groups = ChartRenderer.#groups(selection);
    ChartRenderer.#removeMissing(groups);
    ChartRenderer.#drawCanvases(view, groups, selection.mode);
  }

  /** @param {HTMLElement} view @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string} mode @returns {void} */
  static #drawCanvases(view, groups, mode) {
    for (const canvas of view.querySelectorAll('canvas.resource-chart')) {
      ChartRenderer.#drawCanvas(canvas, groups, mode);
    }
  }

  /** @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {import('/models/RuntimeView.ts').RuntimeView[]} */
  static #groups(selection) {
    return selection.model?.runtimes ?? [];
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @returns {void} */
  static #removeMissing(groups) {
    const identifiers = new Set(groups.map(
      /** @param {import('/models/RuntimeView.ts').RuntimeView} group @returns {string} */
      group => group.id
    ));
    for (const [identifier, chart] of ChartRenderer.#charts) {
      ChartRenderer.#reconcileChart(identifier, chart, identifiers);
    }
  }

  /** @param {string} identifier @param {import('/models/ChartInstance.ts').ChartInstance} chart @param {Set<string>} identifiers @returns {void} */
  static #reconcileChart(identifier, chart, identifiers) {
    const absent = !identifiers.has(identifier);
    ChartRenderer.#removeAbsent(identifier, chart, absent);
  }

  /** @param {string} identifier @param {import('/models/ChartInstance.ts').ChartInstance} chart @param {boolean} absent @returns {void} */
  static #removeAbsent(identifier, chart, absent) {
    if (!absent) {
      return;
    }
    chart.destroy();
    ChartRenderer.#charts.delete(identifier);
  }

  /** @param {Element} element @param {import('/models/RuntimeView.ts').RuntimeView[]} groups @param {string} mode @returns {void} */
  static #drawCanvas(element, groups, mode) {
    const canvas = /** @type {HTMLCanvasElement} */ (element);
    const found = groups.find(
      /** @param {import('/models/RuntimeView.ts').RuntimeView} item @returns {boolean} */
      item => item.id === canvas.dataset.group
    );
    const group = ChartRenderer.#canvasGroup(found);
    const constructor = ChartRenderer.#chartConstructor();
    ChartRenderer.#validate(/** @type {HTMLElement} */ (canvas.closest('.history-card')), {constructor, canvas, group, mode});
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView | undefined} group @returns {import('/models/RuntimeView.ts').RuntimeView | null} */
  static #canvasGroup(group) {
    return group ?? null;
  }

  /** @returns {import('/models/ChartConstructor.ts').ChartConstructor | undefined} */
  static #chartConstructor() {
    return /** @type {Window & {Chart?: import('/models/ChartConstructor.ts').ChartConstructor}} */ (window).Chart;
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: Element | null, group: import('/models/RuntimeView.ts').RuntimeView | null, mode: string}} input @returns {void} */
  static #validate(view, input) {
    if (![input.constructor, input.canvas, input.group].every(Boolean)) {
      return;
    }
    ChartRenderer.#render(view, input);
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: Element | null, group: import('/models/RuntimeView.ts').RuntimeView | null, mode: string}} input @returns {void} */
  static #render(view, input) {
    const canvas = /** @type {HTMLCanvasElement} */ (input.canvas);
    const group = /** @type {import('/models/RuntimeView.ts').RuntimeView} */ (input.group);
    const projection = ChartRenderer.#projection(group, input.mode);
    ChartRenderer.#renderProjection(view, {...input, group, canvas}, projection);
  }

  /** @param {import('/models/RuntimeView.ts').RuntimeView} group @param {string} mode @returns {import('/models/ChartView.ts').ChartView | undefined} */
  static #projection(group, mode) {
    const found = group.charts.find(
      /** @param {import('/models/ChartView.ts').ChartView} view @returns {boolean} */
      view => view.mode === mode
    );
    return ChartRenderer.#firstProjection(found, group.charts);
  }

  /** @param {import('/models/ChartView.ts').ChartView | undefined} found @param {import('/models/ChartView.ts').ChartView[]} charts @returns {import('/models/ChartView.ts').ChartView | undefined} */
  static #firstProjection(found, charts) {
    return found ?? charts[0];
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: HTMLCanvasElement, group: import('/models/RuntimeView.ts').RuntimeView, mode: string}} input @param {import('/models/ChartView.ts').ChartView | undefined} projection @returns {void} */
  static #renderProjection(view, input, projection) {
    if (projection === undefined) {
      return;
    }
    ChartRenderer.#applyProjection(view, input, projection);
  }

  /** @param {HTMLElement} view @param {{constructor: import('/models/ChartConstructor.ts').ChartConstructor | undefined, canvas: HTMLCanvasElement, group: import('/models/RuntimeView.ts').RuntimeView, mode: string}} input @param {import('/models/ChartView.ts').ChartView} projection @returns {void} */
  static #applyProjection(view, input, projection) {
    const datasets = ChartRenderer.#datasets(projection);
    const options = ChartRenderer.#options(projection);
    ChartRenderer.#update(input.canvas, /** @type {import('/models/ChartConstructor.ts').ChartConstructor} */ (input.constructor), {datasets, options});
    ChartRenderer.#count(input.canvas.dataset.group, projection.measuredCount);
    ChartRenderer.#empty(view, projection);
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {import('/models/ChartDataset.ts').ChartDataset[]} */
  static #datasets(projection) {
    /** @type {import('/models/ChartDataset.ts').ChartDataset[]} */
    const datasets = [];
    for (const series of projection.series) {
      datasets.push(ChartRenderer.#dataset(series, projection));
    }
    return datasets;
  }

  /** @param {string} identifier @returns {string} */
  static #color(identifier) {
    return ChartRenderer.#COLORS.get(identifier) ?? ChartRenderer.#BLUE;
  }

  /** @param {import('/models/ChartSeries.ts').ChartSeries} series @param {import('/models/ChartView.ts').ChartView} projection @returns {import('/models/ChartDataset.ts').ChartDataset} */
  static #dataset(series, projection) {
    const color = ChartRenderer.#color(series.id);
    return {
      label: series.label,
      data: series.points,
      borderColor: color,
      backgroundColor: color + '12',
      borderWidth: ChartRenderer.#SERIES_BORDER_WIDTH,
      pointRadius: ChartRenderer.#pointRadius(projection.sampleCount),
      pointHoverRadius: ChartRenderer.#HOVER_RADIUS,
      pointHitRadius: ChartRenderer.#HIT_RADIUS,
      spanGaps: false,
      tension: 0,
      fill: projection.mode !== ChartRenderer.#NETWORK_MODE
    };
  }

  /** @param {number} count @returns {number} */
  static #pointRadius(count) {
    if (count < ChartRenderer.#MINIMUM_POINTS) {
      return 3;
    }
    return 0;
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {Record<string, unknown>} */
  static #options(projection) {
    const mode = projection.mode;
    const unit = projection.unit;
    return {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      normalized: false,
      interaction: {mode: 'nearest', axis: 'x', intersect: false},
      layout: {padding: {top: ChartRenderer.#TOP_PADDING, right: 5}},
      plugins: {
        legend: {
          display: [ChartRenderer.#NETWORK_MODE, ChartRenderer.#DISK_MODE].includes(mode),
          labels: {color: '#ccd2e0', font: {size: ChartRenderer.#FONT_SIZE}, boxWidth: 14}
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
            title: ChartRenderer.#tooltipTime,
            label: /** @param {{dataset: {label: string}, parsed: {y: number}}} item @returns {string} */ item => ChartRenderer.#tooltipValue(item, unit)
          }
        }
      },
      scales: {
        x: ChartRenderer.#timeAxis(projection),
        y: ChartRenderer.#valueAxis(mode, unit)
      }
    };
  }

  /** @param {import('/models/ChartView.ts').ChartView} projection @returns {Record<string, unknown>} */
  static #timeAxis(projection) {
    return {
      type: 'linear',
      min: projection.windowStart,
      max: projection.windowEnd,
      grid: {color: ChartRenderer.#GRID_COLOR},
      ticks: {color: ChartRenderer.#TIME_COLOR, maxTicksLimit: ChartRenderer.#TICK_LIMIT, font: {size: ChartRenderer.#FONT_SIZE}, callback: ChartRenderer.#timeTick},
      title: {display: true, text: 'Local time · last two minutes', color: ChartRenderer.#TIME_COLOR, font: {size: ChartRenderer.#FONT_SIZE}}
    };
  }

  /** @param {string} mode @param {string | undefined} unit @returns {Record<string, unknown>} */
  static #valueAxis(mode, unit) {
    return {
      beginAtZero: mode !== 'memory',
      suggestedMax: ChartRenderer.#maximum(mode),
      grid: {color: ChartRenderer.#GRID_COLOR},
      ticks: {color: ChartRenderer.#VALUE_COLOR, maxTicksLimit: ChartRenderer.#TICK_LIMIT, font: {size: ChartRenderer.#FONT_SIZE}},
      title: {display: true, text: unit, color: ChartRenderer.#VALUE_COLOR, font: {size: ChartRenderer.#FONT_SIZE}}
    };
  }

  /** @param {string} mode @returns {number | undefined} */
  static #maximum(mode) {
    if (mode === ChartRenderer.#CPU_MODE) {
      return ChartRenderer.#CPU_MAXIMUM;
    }
    return undefined;
  }

  /** @param {Array<{parsed: {x: number}}>} items @returns {string} */
  static #tooltipTime(items) {
    return new Date(items[0].parsed.x * ChartRenderer.#MILLISECONDS).toLocaleTimeString();
  }

  /** @param {{dataset: {label: string}, parsed: {y: number}}} item @param {string | undefined} unit @returns {string} */
  static #tooltipValue(item, unit) {
    return item.dataset.label + ': ' + Number(item.parsed.y).toFixed(ChartRenderer.#VALUE_DECIMALS) + ' ' + unit;
  }

  /** @param {number | string} value @returns {string} */
  static #timeTick(value) {
    const time = new Date(Number(value) * ChartRenderer.#MILLISECONDS);
    return time.toLocaleTimeString([], {
      hour: ChartRenderer.#TWO_DIGIT_TIME,
      minute: ChartRenderer.#TWO_DIGIT_TIME,
      second: ChartRenderer.#TWO_DIGIT_TIME
    });
  }

  /** @param {HTMLCanvasElement} canvas @param {import('/models/ChartConstructor.ts').ChartConstructor} constructor @param {{datasets: import('/models/ChartDataset.ts').ChartDataset[], options: Record<string, unknown>}} values @returns {void} */
  static #update(canvas, constructor, values) {
    const identifier = ChartRenderer.#identifier(canvas.dataset.group);
    const previous = ChartRenderer.#charts.get(identifier);
    ChartRenderer.#replaceCanvas(canvas, previous?.canvas);
    ChartRenderer.#updateOrCreate(canvas, constructor, {...values, identifier}, previous);
  }

  /** @param {string | undefined} identifier @returns {string} */
  static #identifier(identifier) {
    return identifier ?? '';
  }

  /** @param {HTMLCanvasElement} canvas @param {HTMLCanvasElement | undefined} previous @returns {void} */
  static #replaceCanvas(canvas, previous) {
    const replace = [previous !== undefined, canvas !== previous].every(Boolean);
    if (!replace) {
      return;
    }
    canvas.parentNode?.replaceChild(/** @type {HTMLCanvasElement} */ (previous), canvas);
  }

  /** @param {HTMLCanvasElement} canvas @param {import('/models/ChartConstructor.ts').ChartConstructor} constructor @param {{datasets: import('/models/ChartDataset.ts').ChartDataset[], options: Record<string, unknown>, identifier: string}} values @param {import('/models/ChartInstance.ts').ChartInstance | undefined} previous @returns {void} */
  static #updateOrCreate(canvas, constructor, values, previous) {
    if (!previous) {
      const chart = new constructor(canvas, {type: 'line', data: {datasets: values.datasets}, options: values.options});
      ChartRenderer.#charts.set(values.identifier, chart);
      return;
    }
    previous.data = {datasets: values.datasets};
    previous.options = values.options;
    previous.update('none');
  }

  /** @param {string | undefined} identifier @param {number} points @returns {void} */
  static #count(identifier, points) {
    ChartRenderer.#setCount(ChartRenderer.#identifier(identifier), points);
  }

  /** @param {string} identifier @param {number} points @returns {void} */
  static #setCount(identifier, points) {
    const chart = ChartRenderer.#charts.get(identifier);
    if (!chart) {
      return;
    }
    chart.canvas.dataset.points = String(points);
  }

  /** @param {HTMLElement} view @param {import('/models/ChartView.ts').ChartView} projection @returns {void} */
  static #empty(view, projection) {
    const empty = /** @type {HTMLElement | null} */ (view.querySelector('.chart-empty'));
    ChartRenderer.#showEmpty(empty, projection.empty, projection.emptyText);
  }

  /** @param {HTMLElement | null} empty @param {boolean} noData @param {string} message @returns {void} */
  static #showEmpty(empty, noData, message) {
    const text = ChartRenderer.#emptyElement(empty);
    ChartRenderer.#replaceText(text, message);
    ChartRenderer.#hideEmpty(text, noData);
  }

  /** @param {HTMLElement | null} element @param {string} text @returns {void} */
  static #replaceText(element, text) {
    const change = [element !== null, element?.textContent !== text].every(Boolean);
    if (!change) {
      return;
    }
    /** @type {HTMLElement} */ (element).textContent = text;
  }

  /** @param {HTMLElement | null} element @returns {HTMLElement | null} */
  static #emptyElement(element) {
    return element;
  }

  /** @param {HTMLElement | null} element @param {boolean} noData @returns {void} */
  static #hideEmpty(element, noData) {
    if (!element) {
      return;
    }
    element.hidden = !noData;
  }

  /** @returns {void} */
  static #dispose() {
    for (const chart of ChartRenderer.#charts.values()) {
      chart.destroy();
    }
    ChartRenderer.#charts.clear();
  }
}
