// Prototype trace recorder: raw Chrome DevTools Protocol, no dependencies (Node 22+).
//
//   node tools/record.mjs <page.html|url> <out.json> [--attach] [--click <selector>]
//
// By default it launches headless google-chrome-stable itself. With --attach it connects
// to a Chrome you started yourself:
//   google-chrome-stable --headless=new --remote-debugging-port=9333 --user-data-dir=.chrome-profile
//
// This script is temporary: recording will move into the Rust CLI later (see docs/specs).
import { spawn } from 'node:child_process';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const positional = args.filter((a, i) => !a.startsWith('--') && args[i - 1] !== '--click');
const [target, outPath] = positional;
if (!target || !outPath) {
  console.error('usage: node tools/record.mjs <page.html|url> <out.json> [--attach] [--click <selector>]');
  process.exit(1);
}
const url = /^(https?|file):/.test(target) ? target : 'file://' + resolve(target);
const clickSelector = opt('--click', '#btn');
const port = 9333;

// Categories that make the trace explain *why* things happened, not only *what*.
const CATEGORIES = [
  'devtools.timeline', 'v8.execute', 'blink.user_timing', 'loading', 'latencyInfo',
  'disabled-by-default-devtools.timeline',
  'disabled-by-default-devtools.timeline.frame',
  'disabled-by-default-devtools.timeline.stack',                  // JS stacks on Layout / style events
  'disabled-by-default-devtools.timeline.invalidationTracking',   // per-node invalidation reasons
  'disabled-by-default-devtools.timeline.layers',
  'disabled-by-default-devtools.timeline.picture',
  'disabled-by-default-blink.debug',                              // SelectorStats
  'disabled-by-default-devtools.screenshot',
  'disabled-by-default-v8.cpu_profiler',
];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let chrome = null;
if (!args.includes('--attach')) {
  const profile = mkdtempSync(join(tmpdir(), 'jankscope-'));
  chrome = spawn('google-chrome-stable', [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', 'about:blank',
  ], { stdio: 'ignore' });
}

let targets = [];
for (let i = 0; i < 50 && !targets.length; i++) {
  try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); } catch {}
  if (!targets.length) await sleep(200);
}
const page = targets.find((t) => t.type === 'page');
if (!page) {
  console.error(`no Chrome on port ${port}; start one manually and pass --attach`);
  chrome?.kill();
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r));

let nextId = 0;
const pending = new Map();
const events = [];
let onComplete;
const complete = new Promise((r) => (onComplete = r));
ws.addEventListener('message', (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
  else if (msg.method === 'Tracing.dataCollected') events.push(...msg.params.value);
  else if (msg.method === 'Tracing.tracingComplete') onComplete();
});
const send = (method, params = {}) => new Promise((r) => {
  const id = ++nextId;
  pending.set(id, r);
  ws.send(JSON.stringify({ id, method, params }));
});

await send('Page.enable');
await send('Page.navigate', { url });
await sleep(1000);

await send('Tracing.start', {
  traceConfig: { includedCategories: CATEGORIES, excludedCategories: ['*'] },
  transferMode: 'ReportEvents',
});
await sleep(300);
await send('Runtime.evaluate', { expression: `document.querySelector(${JSON.stringify(clickSelector)})?.click()` });
await sleep(1500);
await send('Tracing.end');
await complete;

writeFileSync(outPath, JSON.stringify({ traceEvents: events }));
console.log(`${events.length} events -> ${outPath}`);
ws.close();
chrome?.kill();
