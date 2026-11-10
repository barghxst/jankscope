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

**Context.** A prototype trace of `fixtures/layout-thrashing.html` (≈6.5 MB, ≈10k events) showed that
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
