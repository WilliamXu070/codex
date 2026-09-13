# Fork API additions

- `context/index` — experimental; build William's persistent heuristic context tree from a local directory and store it under `CODEX_HOME/context`; emits `context/indexProgress` and `context/indexComplete`.
- `context/query` — experimental; search the persistent context tree and return up to `maxResults` matching node summaries.
- `context/node/get` — experimental; return one context node together with its ancestry and related nodes.
- `context/domains/list` — experimental; list the top-level domains in the persistent context tree.

In User approval mode (`approvalsReviewer: "user"`), async Guardian scoring and
prewarming are skipped, and ordinary `node_repl.js` execution confirmations are
accepted automatically. Separate sensitive-action checks and requests for user
input keep their existing behavior.
