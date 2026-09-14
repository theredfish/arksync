// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createController } from '../src/controller.js';

const config = {
  label: 'Water temperature', unit: '°C', precision: 1, minimum: 0, maximum: 40,
  historyMinimum: 27.75, historyMaximum: 28.25,
  dark: true, background: '#222', text: '#fff', mutedText: '#ddd', grid: '#555', color: '#7db9e8',
};
const snapshot = { timestamps: [1000, 2500, 6000], values: [28, 28.1, 28.2] };

function fixture(kind = 'history', width = 400) {
  const host = { clientWidth: width, clientHeight: 300, isConnected: true, dataset: {} };
  const frames = new Map();
  let nextFrame = 0;
  let observer;
  const calls = { init: [], options: [], resize: [], disposed: 0, disconnected: 0, removedMotion: 0 };
  const listeners = new Map();
  let zoom = { start: 0, end: 100, startValue: 1000, endValue: 6000 };
  const chart = {
    on: (name, fn) => listeners.set(name, fn),
    off: name => listeners.delete(name),
    setOption: (option, opts) => calls.options.push([option, opts]),
    resize: size => calls.resize.push(size),
    getOption: () => ({ dataZoom: [zoom] }),
    dispose: () => calls.disposed++,
  };
  const motion = {
    matches: false,
    addEventListener: (_, fn) => listeners.set('motion', fn),
    removeEventListener: () => { calls.removedMotion++; listeners.delete('motion'); },
  };
  const environment = {
    matchMedia: () => motion,
    requestAnimationFrame: fn => { const id = ++nextFrame; frames.set(id, fn); return id; },
    cancelAnimationFrame: id => frames.delete(id),
    ResizeObserver: class {
      constructor(fn) { observer = fn; }
      observe() {}
      disconnect() { calls.disconnected++; }
    },
  };
  const engine = { init: (...args) => { calls.init.push(args); return chart; } };
  const controller = createController(host, kind, config, engine, environment);
  const flush = () => {
    const pending = [...frames.values()];
    frames.clear();
    pending.forEach(fn => fn());
  };
  const resize = (width, height = 300) => observer([{ target: host, contentRect: { width, height } }]);
  return { host, controller, calls, frames, listeners, flush, resize, motion,
    zoom: next => { zoom = next; listeners.get('datazoom')(); } };
}

test('streaming data updates do not resize or rebuild chart options', () => {
  const f = fixture();
  f.controller.update(snapshot);
  f.flush();
  assert.equal(f.calls.init.length, 1);
  assert.equal(f.calls.init[0][2].renderer, 'canvas');
  f.calls.options.length = 0;
  f.controller.update({ ...snapshot, values: [29, 30, 31] });
  f.flush();
  assert.equal(f.calls.resize.length, 0);
  assert.equal(f.calls.init.length, 1);
  assert.equal(f.calls.options.length, 1);
  assert.equal(f.calls.options[0][0].tooltip, undefined);
  assert.equal(f.calls.options[0][0].yAxis, undefined);
});

test('same-size observations do nothing; size changes coalesce without setOption', () => {
  const f = fixture();
  f.flush();
  f.calls.options.length = 0;
  f.resize(400);
  f.flush();
  assert.equal(f.calls.resize.length, 0);
  f.resize(450);
  f.resize(500);
  assert.equal(f.frames.size, 1);
  f.flush();
  assert.deepEqual(f.calls.resize, [{ width: 500, height: 300, silent: true, animation: { duration: 0 } }]);
  assert.equal(f.calls.options.length, 0);
});

test('hidden charts initialize only when measurable, retaining the latest data', () => {
  const f = fixture('history', 0);
  f.controller.update(snapshot);
  f.flush();
  assert.equal(f.calls.init.length, 0);
  f.resize(400);
  f.flush();
  assert.equal(f.calls.init.length, 1);
  assert.deepEqual(f.calls.options[1][0].series[0].data, [[1000, 28], [2500, 28.1], [6000, 28.2]]);
});

test('theme changes retain the instance and do not replace data or zoom bounds', () => {
  const f = fixture();
  f.flush();
  f.calls.options.length = 0;
  f.controller.configure(config);
  f.flush();
  assert.equal(f.calls.options.length, 0);
  f.controller.configure({ ...config, dark: false, text: '#111' });
  f.flush();
  assert.equal(f.calls.init.length, 1);
  assert.equal(f.calls.resize.length, 0);
  const options = f.calls.options[0][0];
  assert.equal(options.series[0].data, undefined);
  assert.equal(options.dataZoom[0].start, undefined);
  assert.equal(options.dataZoom[0].startValue, undefined);
});

test('zoom is stored as absolute dates and reset to live explicitly', () => {
  const f = fixture();
  f.controller.update(snapshot);
  f.flush();
  f.zoom({ start: 20, end: 80, startValue: 2000, endValue: 5000 });
  f.flush();
  f.controller.update({ timestamps: [2500, 6000, 9000], values: [28.1, 28.2, 28.3] });
  f.flush();
  assert.deepEqual(f.calls.options.at(-1)[0].dataZoom[0], {
    id: 'time-slider', rangeMode: ['value', 'value'], startValue: 2000, endValue: 5000,
  });
  f.controller.resetZoom();
  f.flush();
  assert.deepEqual(f.calls.options.at(-1)[0].dataZoom[0], {
    id: 'time-slider', rangeMode: ['percent', 'percent'], start: 0, end: 100,
  });
});

test('unchanged gauge values do not trigger animation or redraw', () => {
  const f = fixture('gauge');
  f.controller.update(28.1);
  f.flush();
  const count = f.calls.options.length;
  f.controller.update(28.1);
  f.flush();
  assert.equal(f.calls.options.length, count);
  f.controller.update(null);
  f.flush();
  assert.deepEqual(f.calls.options.at(-1)[0].series[0].data, []);
});

test('reduced motion is respected and can change at runtime', () => {
  const f = fixture('gauge');
  f.flush();
  f.motion.matches = true;
  f.listeners.get('motion')();
  f.flush();
  assert.equal(f.calls.options.at(-1)[0].animation, false);
  assert.equal(f.calls.options.at(-1)[0].series[0].detail.valueAnimation, false);
});

test('cleanup cancels pending work and disposes listeners, observer and chart exactly once', () => {
  const f = fixture();
  f.flush();
  f.controller.update(snapshot);
  f.controller.dispose();
  f.controller.dispose();
  f.resize(800);
  f.controller.update(snapshot);
  f.flush();
  assert.equal(f.frames.size, 0);
  assert.equal(f.calls.disposed, 1);
  assert.equal(f.calls.disconnected, 1);
  assert.equal(f.calls.removedMotion, 1);
  assert.equal(f.listeners.size, 0);
  assert.equal(f.calls.resize.length, 0);
});

test('cleanup before the first frame does not create a chart', () => {
  const f = fixture();
  f.controller.dispose();
  f.flush();
  assert.equal(f.calls.init.length, 0);
  assert.equal(f.frames.size, 0);
});
