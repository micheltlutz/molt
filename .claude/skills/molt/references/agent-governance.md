# Agent governance for this repository

How an agent behaves here: what it may do alone, what needs a yes, and what leaves a trail.

**Provenance.** The `agent-governance-reviewer` page named in the original request
(<https://claudecowork.im/resources/agent-governance-reviewer>) is an install stub that
publishes no rules; it only describes an agent that reviews "safety issues, missing governance
controls… policy enforcement, trust scoring and audit trails". Nothing below is copied from
it. This page is written from how the 1.0 batches were actually run. Where it makes a claim, a
real episode backs it.

## Autonomy levels

| level | examples | rule |
| --- | --- | --- |
| **Free** | read code, run tests/clippy/bench locally, write tests, edit docs and source on a branch, commit locally | just do it; keep each commit to one logical change |
| **Ask once per batch** | create a branch family, open PRs, create issues/labels/milestones, push branches | the user approved the *plan* that lists them; approval of a plan covers the actions it names, not new ones |
| **Ask every time** | `git push --force`, deleting branches/issues/artifacts, publishing to crates.io, regenerating golden fixtures (`MOLT_UPDATE_FIXTURES=1`), installing global tools, changing CI permissions or secrets, merging a PR | name the exact action, wait for the answer, and do not generalise it to the next one |
| **Never** | editing `tests/fixtures/` by hand, adding `unsafe`, putting a token or credential in a file/log/PR body, bypassing a hook or check to get green, `--no-verify` | if it seems necessary, the plan is wrong: stop and say so |

Approval is scoped. "Go ahead with batch A" authorised branches, commits, pushes and PRs; it
did not authorise merging, and the owner merged. A plan approved once lists its own actions
(issues, labels, a project, PRs); an action outside that list needs its own yes. When a user
message arrives mid-task, treat it as a new instruction to fold in, not as consent for
something adjacent.

## Evidence before claims

- "Done" means `./scripts/verify.sh` was run and its output read. If it fails on something
  outside the change (the missing `cargo-deny`), say exactly that.
- A performance claim carries a benchmark (before and after, including regressions) or an
  allocation count. A security claim carries the hostile input that used to work and a test
  that now fails it. A doc claim was run.
- Report what you did *not* verify. "Confirmed by reading, not by running" is a legitimate,
  honest sentence; "should work" is not evidence.
- When a subagent or a tool reports something, treat it as a lead. Two findings in these
  batches were confirmed by reading the code before being filed (`clean(".")` and the
  temporary-file collision); the dense-escape slowdown was found only because the benchmark
  was run.

## Audit trail

- One logical change per commit; prefixes are
  (`Add:`, `Fix:`, `Change:`, `Update:`, `Docs:`, `Test:`, `Refactor:`, `Perf:`).
- The commit body says **why** and, for performance, the numbers. `Closes #N` links the issue.
- A PR body states: what changed, what was measured, what it deliberately does not do, what
  diverges from the issue and why, and any fixture diff. Breaking changes are labelled
  `breaking` and listed for `MIGRATION.md`.
- The decision, not just the diff, goes into [`decision-log.md`](decision-log.md) when a
  future agent could reasonably undo it.
- Plans live in `docs/plans/`; the issue tracker (milestone `1.0.0`, project "Molt 1.0.0")
  is the source of truth for scope.

## Safety around irreversible steps

- Chain irreversible steps with `&&`, never `;` (L17).
- Before deleting or overwriting, look at the target.
- A destructive function is tested through its **pure guard**, never by pointing it at the real
  working directory (L5).
- Pushing publishes. Cache and mirrors may keep it even if deleted. Push only what you would
  be content to see indexed.
- Secrets: the release job holds `CARGO_REGISTRY_TOKEN`. Never echo it, never write it down,
  never put it in a test.
- Do not mention user email or names of people in commits, docs or issues beyond what the
  project already records (`CODEOWNERS`, `authors`).

## Working with subagents

Spawn a subagent when work is independent and its output is a summary (a codebase survey, a
review of a distinct area), not for a lookup you can do in two commands. Brief it as a peer:
the goal, what is ruled out, which files matter, and that it is read-only if it should be.
Never delegate the decision. Findings come back in a confident voice whether or not they are
right: verify the ones you will act on.

## Stop and ask when

- A change needs `unsafe`, a new runtime dependency, or a new public trait.
- A fixture would have to change.
- A fixture-encoded rule (AGENTS.md 3, 5, 6, 7, 8, 9, 10) conflicts with a security fix. Propose the
  contextual option (L3) and let the owner choose.
- The issue and the evidence disagree (L13). Do the evidence-backed thing and say so, or ask.
- You are about to do something the plan does not list.
