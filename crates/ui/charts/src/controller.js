// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
import { appearance, historyDataOption, historyPoints, SERIES_ID } from './options.js';

export function createController(host, kind, initialConfig, engine, environment = window) {
  let chart;
  let disposed = false;
  let frame = null;
  let config = initialConfig;
  let configKey = JSON.stringify(config);
  let styleDirty = true;
  let dataDirty = true;
  let snapshot = null;
  let selection = null;
  let size = { width: host.clientWidth, height: host.clientHeight };
  let renderedSize;
  const reducedMotion = environment.matchMedia('(prefers-reduced-motion: reduce)');

  function schedule() {
    if (!disposed && frame === null) frame = environment.requestAnimationFrame(flush);
  }

  function captureZoom() {
    const zoom = chart.getOption().dataZoom[0];
    selection = zoom.start <= 0 && zoom.end >= 100
      ? null
      : Number.isFinite(zoom.startValue) && Number.isFinite(zoom.endValue)
        ? [zoom.startValue, zoom.endValue] : null;
    // Switch from percentages to absolute dates before a subsequent resize or
    // data update can shift the selected window.
    dataDirty = true;
    schedule();
  }

  function flush() {
    frame = null;
    if (disposed || !host.isConnected || size.width <= 0 || size.height <= 0) return;
    if (!chart) {
      chart = engine.init(host, config.dark ? 'arksync-dark' : 'arksync-light', { ...size, renderer: 'canvas' });
      renderedSize = { ...size };
      if (kind === 'history') chart.on('datazoom', captureZoom);
    } else if (size.width !== renderedSize.width || size.height !== renderedSize.height) {
      chart.resize({ ...size, silent: true, animation: { duration: 0 } });
      renderedSize = { ...size };
    }
    if (styleDirty) {
      chart.setOption(appearance(kind, config, reducedMotion.matches), { silent: true });
      styleDirty = false;
    }
    if (dataDirty) {
      const option = kind === 'history'
        ? historyDataOption(historyPoints(snapshot), selection)
        : { series: [{ id: SERIES_ID, data: Number.isFinite(snapshot) ? [{ value: snapshot }] : [] }] };
      chart.setOption(option, { silent: true });
      dataDirty = false;
    }
  }

  // contentRect measures layout dimensions, unlike getBoundingClientRect which
  // includes the grid's FLIP transforms. The lightweight resize preview never
  // changes the host's size; committing the grid size causes a single resize.
  const observer = new environment.ResizeObserver(entries => {
    const entry = entries.find(entry => entry.target === host);
    if (!entry) return;
    const next = { width: Math.round(entry.contentRect.width), height: Math.round(entry.contentRect.height) };
    if (next.width !== size.width || next.height !== size.height) {
      size = next;
      schedule();
    }
  });
  observer.observe(host);
  const onMotionChange = () => { styleDirty = true; schedule(); };
  reducedMotion.addEventListener('change', onMotionChange);
  schedule();

  return {
    configure(next) {
      const nextKey = JSON.stringify(next);
      if (nextKey === configKey || disposed) return;
      config = next;
      configKey = nextKey;
      styleDirty = true;
      schedule();
    },
    update(next) {
      if (disposed) return;
      // In particular, history updates must not animate an unchanged gauge.
      if (kind === 'gauge' && Object.is(snapshot, next)) return;
      snapshot = next;
      dataDirty = true;
      schedule();
    },
    resetZoom() {
      if (kind !== 'history' || disposed) return;
      selection = null;
      dataDirty = true;
      schedule();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      observer.disconnect();
      reducedMotion.removeEventListener('change', onMotionChange);
      if (frame !== null) environment.cancelAnimationFrame(frame);
      if (chart) {
        if (kind === 'history') chart.off('datazoom', captureZoom);
        chart.dispose();
      }
      snapshot = null;
    },
  };
}
