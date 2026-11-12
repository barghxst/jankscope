# Test traces

Small traces cut from full recordings, committed so tests run on any machine.
Full recordings live in `../recorded/` and are not committed; record them with `tools/record.mjs`
(see the top-level README).

Name each trace by what it is for, not by its size. When you add one, add a section below:
the source recording, the exact command, and what the trace covers.

## parse-edge-cases.trace.json

Every quirk the parser has to survive, taken from a real Chrome trace. Used by `crates/core/tests/parse.rs`.

**Source:** `fixtures/recorded/layout-thrashing.trace.json`, recorded from `fixtures/pages/layout-thrashing.html`.

**Command** (from the repository root):

```bash
jq '{traceEvents: ([.traceEvents | group_by(.ph)[] | .[:4][]] + [.traceEvents[] | select(.name == "thread_name" and .args.name == "CrRendererMain")] | sort_by(.ts))}' \
  fixtures/recorded/layout-thrashing.trace.json > fixtures/traces/parse-edge-cases.trace.json
```

The first 4 events of each phase, plus the `thread_name` metadata for `CrRendererMain`, sorted by time.

**Covers:**

| Case | Count |
|---|---|
| events in total | 41 |
| phases `X I B b e n s f P` | 4 each |
| `M` metadata, all with `ts: 0` | 5 |
| `B` without a matching `E` | 4 `B`, 0 `E` |
| `id` as a number | 8 |
| `id` as a string (`"0x..."`) | 4 |
| `id2` as an object | 12 |
| `thread_name` for `CrRendererMain` (pid 46120, tid 1) | 1 |

If you regenerate it from a new recording, these numbers change; update the tests in `parse.rs` together with the file.
