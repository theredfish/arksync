// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
import * as echarts from 'echarts/core';
import { GaugeChart, LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent, DataZoomComponent } from 'echarts/components';
import { LegacyGridContainLabel } from 'echarts/features';
import { CanvasRenderer } from 'echarts/renderers';
import chalk from '../../styles/chart_themes/chalk.json';
import walden from '../../styles/chart_themes/walden.json';
import { createController } from './controller.js';

// Keep the production bundle canvas-only. The SVG renderer remains a
// test-only dependency because ECharts' Node/SSR test path renders SVG.
echarts.use([GaugeChart, LineChart, GridComponent, TooltipComponent, DataZoomComponent, LegacyGridContainLabel, CanvasRenderer]);
echarts.registerTheme('arksync-dark', chalk);
echarts.registerTheme('arksync-light', walden);

const controllers = new WeakMap();

export function mount(host, kind, config) {
  dispose(host);
  controllers.set(host, createController(host, kind, config, echarts));
}
export function configure(host, config) { controllers.get(host)?.configure(config); }
export function update(host, snapshot) { controllers.get(host)?.update(snapshot); }
export function resetZoom(host) { controllers.get(host)?.resetZoom(); }
export function dispose(host) {
  controllers.get(host)?.dispose();
  controllers.delete(host);
}
