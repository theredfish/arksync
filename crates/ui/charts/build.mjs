// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
import { build } from 'esbuild';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const output = resolve(root, 'dist');
const result = await build({
  absWorkingDir: root,
  entryPoints: ['src/index.js'],
  outfile: resolve(output, 'arksync-charts.js'),
  bundle: true,
  format: 'iife',
  globalName: 'ArkSyncCharts',
  target: ['es2020'],
  minify: true,
  legalComments: 'inline',
  write: false,
});
const artifacts = result.outputFiles.map(file => [file.path, file.contents]);
for (const [source, destination] of [
  ['echarts/LICENSE', 'licenses/echarts-apache-2.0.txt'],
  ['echarts/NOTICE', 'licenses/echarts-apache-2.0-notice.txt'],
  ['zrender/LICENSE', 'licenses/zrender-bsd-3-clause.txt'],
  ['tslib/LICENSE.txt', 'licenses/tslib-0bsd.txt'],
]) {
  artifacts.push([resolve(output, destination), await readFile(resolve(root, 'node_modules', source))]);
}
for (const [path, contents] of artifacts) {
  if (process.argv.includes('--check')) {
    const existing = await readFile(path);
    if (!existing.equals(Buffer.from(contents))) {
      throw new Error(`${path} is stale; run npm run build in crates/ui/charts`);
    }
  } else {
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, contents);
  }
}
const bundle = result.outputFiles.find(file => file.path.endsWith('.js'));
console.log(`Charts bundle ${process.argv.includes('--check') ? 'verified' : 'built'} (${bundle.contents.length} bytes).`);
