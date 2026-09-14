// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export const SERIES_ID = 'measurement';
export const ZOOM_IDS = ['time-slider', 'time-inside'];

const formatValue = (value, config) => Number.isFinite(value)
  ? `${value.toFixed(config.precision).replace('.', ',')} ${config.unit}`
  : '—';

// Use real timestamps, not evenly spaced category labels. Preserve missing
// measurements as gaps, and never clamp industrial readings to a demo range.
export function historyPoints(snapshot) {
  if (!snapshot) return [];
  const points = new Map();
  for (let index = 0; index < snapshot.values.length; index++) {
    const timestamp = snapshot.timestamps[index];
    if (!Number.isFinite(timestamp)) continue;
    const value = snapshot.values[index];
    points.set(timestamp, Number.isFinite(value) ? value : null);
  }
  return [...points].sort((a, b) => a[0] - b[0]);
}

export function zoomOptions(selection) {
  return ZOOM_IDS.map(id => selection
    ? { id, rangeMode: ['value', 'value'], startValue: selection[0], endValue: selection[1] }
    : { id, rangeMode: ['percent', 'percent'], start: 0, end: 100 });
}

export function historyDataOption(points, selection) {
  // Keep an inspected interval in place even when the live buffer rolls past
  // it. Missing history stays empty; a frontend zoom is not a backend query.
  const minimum = points[0]?.[0];
  const maximum = points[points.length - 1]?.[0];
  return {
    xAxis: {
      min: selection ? Math.min(selection[0], minimum ?? selection[0]) : minimum ?? null,
      max: selection ? Math.max(selection[1], maximum ?? selection[1]) : maximum ?? null,
    },
    series: [{ id: SERIES_ID, data: points }],
    dataZoom: zoomOptions(selection),
  };
}

// This patch deliberately contains no data or selected time range. Changing
// the theme must not replace the instance or reset the user's exploration.
export function appearance(kind, config, reducedMotion) {
  const { background, text, mutedText, grid, color } = config;
  const tooltip = {
    renderMode: 'richText',
    confine: true,
    backgroundColor: background,
    borderColor: grid,
    textStyle: { color: text },
    valueFormatter: value => formatValue(value, config),
    transitionDuration: 0,
  };
  const common = {
    backgroundColor: 'transparent',
    textStyle: { color: mutedText, fontFamily: 'sans-serif' },
    animation: kind === 'gauge' && !reducedMotion,
    animationDuration: 0,
    animationDurationUpdate: kind === 'gauge' && !reducedMotion ? 180 : 0,
    animationEasingUpdate: 'cubicOut',
  };
  if (kind === 'gauge') {
    return {
      ...common,
      tooltip: { ...tooltip, trigger: 'item' },
      series: [{
        id: SERIES_ID,
        type: 'gauge',
        name: config.label,
        min: config.minimum,
        max: config.maximum,
        splitNumber: 5,
        center: ['50%', '52%'],
        radius: '82%',
        axisLine: { lineStyle: { width: 12, color: [[1, grid]] } },
        progress: { show: true, width: 12, itemStyle: { color } },
        axisTick: { distance: 3, length: 5, lineStyle: { color: mutedText, width: 1 } },
        splitLine: { distance: 3, length: 11, lineStyle: { color: mutedText, width: 2 } },
        axisLabel: { distance: 20, color: mutedText, fontSize: 12 },
        pointer: { width: 6, length: '60%', itemStyle: { color } },
        anchor: { show: true, size: 10, itemStyle: { color, borderColor: color } },
        title: { show: false },
        detail: {
          color: text,
          fontSize: 30,
          fontWeight: 600,
          offsetCenter: [0, '70%'],
          valueAnimation: !reducedMotion,
          formatter: value => formatValue(value, config),
        },
        emphasis: { disabled: true },
      }],
    };
  }
  return {
    ...common,
    tooltip: { ...tooltip, trigger: 'axis', axisPointer: { type: 'cross', animation: false } },
    grid: { left: 16, right: 24, top: 34, bottom: 58, containLabel: true },
    xAxis: {
      type: 'time',
      boundaryGap: [0, 0],
      splitNumber: 4,
      axisLabel: { color: mutedText, fontSize: 10, hideOverlap: true },
      axisLine: { lineStyle: { color: grid } },
      axisTick: { show: false },
      splitLine: { show: false },
      axisPointer: { label: { backgroundColor: grid } },
    },
    yAxis: {
      type: 'value',
      scale: true,
      // Keep the useful engineering range but expand for genuine excursions.
      // This avoids both a meaningless 0-based scale and clipped alarm values.
      min: extent => Math.min(config.historyMinimum, extent.min),
      max: extent => Math.max(config.historyMaximum, extent.max),
      splitNumber: 4,
      axisLabel: {
        color: mutedText,
        fontSize: 11,
        formatter: value => Number(value).toFixed(config.precision).replace('.', ','),
      },
      axisLine: { show: false },
      axisTick: { show: false },
      splitLine: { lineStyle: { color: grid, opacity: 0.6 } },
      axisPointer: { label: { backgroundColor: grid, precision: config.precision } },
    },
    series: [{
      id: SERIES_ID,
      name: config.label,
      type: 'line',
      showSymbol: false,
      // Do not interpolate extrema that were never measured.
      smooth: false,
      connectNulls: false,
      sampling: 'lttb',
      lineStyle: { color, width: 2 },
      itemStyle: { color },
      emphasis: { disabled: true },
    }],
    dataZoom: [
      {
        id: ZOOM_IDS[0], type: 'slider', xAxisIndex: 0,
        bottom: 12, left: 50, right: 24, height: 20,
        realtime: false, throttle: 50,
        showDetail: false, brushSelect: true,
        borderColor: grid, backgroundColor: 'transparent',
        fillerColor: config.dark ? 'rgba(125,185,232,0.16)' : 'rgba(63,120,168,0.16)',
        handleStyle: { color: mutedText, borderColor: grid },
        moveHandleStyle: { color: mutedText },
        dataBackground: { lineStyle: { color: grid }, areaStyle: { color: grid, opacity: 0.15 } },
        selectedDataBackground: { lineStyle: { color }, areaStyle: { color, opacity: 0.15 } },
        textStyle: { color: mutedText },
      },
      {
        id: ZOOM_IDS[1], type: 'inside', xAxisIndex: 0,
        throttle: 50, zoomOnMouseWheel: 'ctrl',
        moveOnMouseWheel: false, moveOnMouseMove: true,
      },
    ],
  };
}
