# jankscope

Explains *why* a page janks, not just *that* it does.

DevTools show you a flame chart. jankscope reads the same Chrome trace and tells the story:

```
Frame 14: 48 ms (budget 16 ms)
  click → classList.toggle('active') on #grid       Filters.tsx:18
  → 400 × .card invalidated (selector .grid.active .card)
  → forced synchronous layout ×400, 137 ms          useAutoScroll.ts:22
  fix: batch the reads before the writes
```

Built for two readers:

- **humans**: a readable per-frame explanation;
- **coding agents**: an MCP server and a skill that run *record → fix → re-record → compare*,
  so the agent can show that a fix actually helped.

> Status: early. Nothing works yet. Work goes one spec at a time, see [`docs/specs/`](docs/specs/).

## Layout

```
crates/core   Rust library: trace → frame model → findings
crates/cli    `jankscope` binary
tools/        temporary prototypes (Node trace recorder)
fixtures/     test pages, committed test traces, local recordings
docs/specs/   one spec per task, done in order
DECISIONS.md  why things are the way they are
```

## Recording a trace (prototype)

```bash
node tools/record.mjs fixtures/pages/layout-thrashing.html fixtures/recorded/layout-thrashing.trace.json
```

If Chrome fails to start from Node, start it yourself and pass `--attach`:

```bash
google-chrome-stable --headless=new --remote-debugging-port=9333 --user-data-dir=.chrome-profile
```

Full recordings are large and are not committed: they live in `fixtures/recorded/`, which git ignores.
Regenerate them from `fixtures/pages/*.html`.

Tests use small traces cut from those recordings. They live in `fixtures/traces/` and are committed;
see [`fixtures/traces/README.md`](fixtures/traces/README.md) for where each one comes from.

## Roadmap

1. Core + CLI for the web: forced layout, wide style invalidation, unnecessary paints, layout shift
2. `compare` with statistics: did the fix actually help, or is it noise?
3. MCP server + skill: an agent finds and fixes jank on its own
4. Viewer for humans (WASM core in the browser)
5. React Native: JS via the Metro inspector, then native (Kotlin / Swift)
