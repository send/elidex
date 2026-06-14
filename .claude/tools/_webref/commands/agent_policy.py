"""`agent-policy` subcommand — print the webref drift workflow contract."""
from __future__ import annotations

import argparse


POLICY = """# webref Agent Policy

Use ordinary lookup commands (`heading`, `dfn`, `idl`, `css`, `aoid`, `body`)
for normal citation verification. Do not run drift tooling for every lookup.

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

Run `agent-brief` when:
- `diff` has non-zero semantic changes;
- docs, plan memos, code comments, or docstrings cite affected sections;
- a Coding Agent is expected to update elidex artifacts from the spec change list.

Run `refresh` for intentional upstream refreshes. It fetches a new snapshot,
compares it with the previous saved snapshot, and prints the next command to run
when semantic drift is detected.
"""


def cmd_agent_policy(_args: argparse.Namespace) -> None:
    print(POLICY)
