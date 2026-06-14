# webref Drift Tooling Design

`webref` is maintained inside elidex for now, but its drift-detection core
should stay generic enough to move to a standalone repository later. elidex
specific behavior belongs in thin adapter commands.

## Background

The original `webref` CLI was a lookup helper for spec citation verification:
headings, dfns, IDL fragments, CSS metadata, TC39 AO lookup, and section body
extraction. Its HTTP cache stores raw upstream responses with conditional GET.
That is useful for performance, but raw cache updates are not a reliable
semantic signal:

- upstream JSON field order or generated metadata can change without spec drift;
- spec prose can move chapters without changing a citation target;
- section numbers, titles, anchors, IDL, CSS metadata, and AO mappings drift at
  different grains;
- Coding Agents need an actionable list of affected docs/code, not a byte diff.

Therefore drift tooling compares normalized semantic inventories, not raw HTTP
cache bodies.

## Architecture

Keep the boundary explicit:

- Generic core:
  - upstream fetch/cache;
  - semantic inventory construction;
  - semantic diff classification;
  - stable JSON output schema.
- elidex adapter:
  - repository citation scanning;
  - review/plan workflow wording;
  - impacted `docs/` / `crates/` path heuristics;
  - elidex-specific agent briefs.

Current generic modules:

- `inventory.py` builds normalized snapshots for headings, dfns, TC39 clauses,
  and TC39 AO links.
- `diff.py` compares two snapshots and categorizes changes.
- `commands/snapshot.py` exposes snapshot generation.
- `commands/diff.py` exposes human and JSON diff output.
- `commands/agent_policy.py` exposes the agent-facing workflow contract.
- `commands/agent_brief.py` scans elidex paths for citations affected by a
  semantic diff.
- `commands/refresh.py` captures a new snapshot and compares it with the prior
  saved snapshot.

## Commands

Current:

```sh
.claude/tools/webref snapshot html --output /tmp/html-old.json
.claude/tools/webref snapshot html --output /tmp/html-new.json
.claude/tools/webref diff /tmp/html-old.json /tmp/html-new.json
.claude/tools/webref diff /tmp/html-old.json /tmp/html-new.json --format json
.claude/tools/webref agent-policy
.claude/tools/webref agent-brief /tmp/html-old.json /tmp/html-new.json --paths docs crates
.claude/tools/webref refresh html
```

The first `agent-brief` implementation intentionally uses conservative
substring matching over selected text files. That keeps it dependency-free and
useful as a broad impact queue, but findings still need agent judgment before
editing docs or code.

## Agent Workflow

Agents should not run snapshot/diff for every ordinary citation lookup. Use the
drift commands when the task is about cache refresh, broad spec citation work,
or updating documentation/code after upstream specification changes.

Run `snapshot` when:

- intentionally refreshing upstream webref / TC39 data;
- starting a spec-citation-heavy documentation or implementation update;
- preparing to review broad citation churn;
- capturing a before/after state for a known upstream spec update.

Run `diff` when:

- two snapshots are available;
- a cache refresh may imply semantic drift;
- a PR changes saved snapshots or webref drift tooling;
- a human or agent needs a machine-readable change list.

Run `agent-brief` when available and:

- `diff` has non-zero semantic changes;
- docs, plan memos, code comments, or docstrings cite affected sections;
- a Coding Agent is expected to update elidex artifacts from the spec change
  list.

Run `refresh` when available as the preferred high-level command for intentional
upstream refreshes. It should hide the mechanical sequence:

1. fetch upstream data;
2. write a new snapshot;
3. diff against the previous snapshot;
4. generate or suggest an agent brief if the diff is non-zero.

## Diff Categories

The first implementation classifies:

- `added`: a new stable key appears;
- `removed`: a stable key disappears;
- `renumbered`: a heading keeps its key but changes section number;
- `retitled`: a heading keeps its key but changes title;
- `moved`: an item keeps its key but changes href;
- `changed`: other stable-key field changes.

Future extensions should add new categories only when they remove meaningful
agent work, for example `idl_signature_changed`, `css_metadata_changed`, or
`prose_changed`.

## Externalization Criteria

Keep the tool in elidex until the workflow hardens through real review and
documentation updates. Consider a separate repository once several of these are
true:

- the generic snapshot/diff schema is stable;
- at least three commands are useful outside elidex;
- webref changes naturally deserve review independent of engine changes;
- CI needs golden upstream fixtures or release packaging;
- another project wants to consume the tool without elidex's review workflow.

Until then, keep new generic behavior free of elidex-specific file paths and
put elidex policy in adapter commands or documentation.
