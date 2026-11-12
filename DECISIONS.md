# Decisions

Short records: what was decided, why, and what else was considered. Newest at the bottom.

## 001. The niche: causality and proof, not metrics (2026-11-10)

**Context.** Agent tooling for performance already exists: `chrome-devtools-mcp` (Chrome DevTools
Insights for agents, mostly page load), `react-render-profile-mcp` and React Scan MCP servers
(React re-renders), web-perf skills.

**Decision.** jankscope covers what they do not:
1. causal chains: JS line → invalidation → style / layout / paint cost → fix;
2. before / after comparison with statistics, so an agent can prove a fix worked;
3. React Native, where no agent tooling for performance was found.

## 002. The data source is the Chrome trace with hidden categories (2026-11-10)

**Context.** A prototype trace of `fixtures/pages/layout-thrashing.html` (≈6.5 MB, ≈10k events) showed that
`disabled-by-default-devtools.timeline.invalidationTracking` and `disabled-by-default-blink.debug` give:
per-node invalidation reasons with JS stacks, `Layout` events with a `stackTrace` (forced layout),
`SelectorStats` per selector, `LayoutShift` with old and new rects.

**Decision.** Build on trace events, not on screen scraping DevTools or on DevTools Insights.

**Cost.** These categories slow the page down, so they are enabled only while recording.
The trace format is not formally documented and changes between Chrome versions; the reference is
the open-source DevTools frontend.

## 003. Rust core, thin wrappers around it (2026-11-10)

**Decision.** All analysis lives in `crates/core`. CLI, MCP server, WASM viewer and the skill are thin
layers over it, with no logic of their own.

**Why.** Real traces are hundreds of MB; the same core has to run in the CLI, in CI and in the browser (WASM).
One implementation means one place for bugs.

## 004. Recording stays in Node for now (2026-11-10)

**Decision.** `tools/record.mjs` (raw CDP, no dependencies) records traces until the analysis works.
Recording moves to Rust later.

**Why.** Recording is not the interesting part yet; analysis is. Don't block on it.

## 005. Trace model: `f64` time, untyped `args`, unknown phases kept (2026-11-12)

**Decision.** `ts` and `dur` are `f64` microseconds. `args` stays a `serde_json::Value`. Unknown phases parse
into `Phase::Other(String)` instead of failing. `id` is a number-or-string enum, kept as written.

**Why.** The trace format allows fractional timestamps even though Chrome writes integers. Every event has its
own `args` shape; we type only the ones an analysis needs, in later specs. A new phase from Chrome must not
break parsing of the whole trace. `id` arrives both as `0` and as `"0x1"`; normalising it is left until we
actually match events by id.

## 006. Fixtures are split by folder, not by name pattern (2026-11-12)

**Decision.** `fixtures/pages/` holds test pages, `fixtures/traces/` holds small committed test traces named
by purpose (`parse-edge-cases`), `fixtures/recorded/` holds full recordings and is the only ignored folder.
`fixtures/traces/README.md` records the source and the exact `jq` command for each trace.

**Why.** Ignoring by pattern needed a `!` exception per committed trace, and a forgotten one silently leaves
a fixture out of git, so tests pass only on the machine that made it. A folder rule needs no upkeep.
