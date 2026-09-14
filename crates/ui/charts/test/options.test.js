// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import { DataZoomComponent, GridComponent } from 'echarts/components';
import { LegacyGridContainLabel } from 'echarts/features';
import { SVGRenderer } from 'echarts/renderers';
import { historyPoints, historyDataOption, appearance } from '../src/options.js';

echarts.use([LineChart, DataZoomComponent, GridComponent, LegacyGridContainLabel, SVGRenderer]);

test('irregular timestamps, duplicate samples, missing values and out-of-range readings', () => {
  assert.deepEqual(historyPoints({
    timestamps: [6000, 1000, 3000, 1000, NaN, 7000],
    values: [110, 5, null, -15, 10, Infinity, 12],
  }), [[1000, -15], [3000, null], [6000, 110], [7000, null]]);
  assert.deepEqual(historyPoints(null), []);
  assert.deepEqual(historyPoints({ timestamps: [], values: [5] }), []);
});

test('real ECharts retains the selected interval when the live window rolls forward', () => {
  const chart = echarts.init(null, null, { renderer: 'svg', ssr: true, width: 500, height: 300 });
  try {
    const config = { label: 'pH', unit: 'pH', precision: 2, dark: true, color: '#f99', grid: '#555', text: '#fff', mutedText: '#ccc', historyMinimum: 6.77, historyMaximum: 6.81 };
    const option = appearance('history', config, false);
    // The SSR test needs no tooltip renderer.
    delete option.tooltip;
    chart.setOption(option);
    chart.setOption(historyDataOption([[1000, 6.8], [2000, 6.9], [6000, 7]], null));
    chart.dispatchAction({ type: 'dataZoom', start: 20, end: 80 });
    const zoom = chart.getOption().dataZoom[0];
    const selection = [zoom.startValue, zoom.endValue];
    chart.setOption(historyDataOption([[6000, 7], [9000, 7.1]], selection));
    const selected = chart.getOption().dataZoom[0];
    assert.deepEqual([selected.startValue, selected.endValue], selection);
    assert.match(chart.renderToSVGString(), /<svg/);
    chart.setOption(historyDataOption([[6000, 7], [9000, 7.1]], null));
    const live = chart.getOption().dataZoom[0];
    assert.deepEqual([live.start, live.end, live.startValue, live.endValue], [0, 100, 6000, 9000]);
  } finally {
    chart.dispose();
  }
});

test('history axis labels use the metric precision', () => {
  const config = {
    label: 'pH', unit: 'pH', precision: 2, dark: true,
    color: '#f99', grid: '#555', text: '#fff', mutedText: '#ccc',
    historyMinimum: 6.77, historyMaximum: 6.81,
  };
  const option = appearance('history', config, false);
  assert.equal(option.yAxis.axisLabel.formatter(6.809999942779541), '6,81');
});
