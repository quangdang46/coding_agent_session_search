# Idea Wizard: Swarm Operations Next Level

Date: 2026-05-08
Author: BronzePeak
Context: `coding_agent_session_search` swarm operations cockpit follow-up planning.

## Phase 1 - Grounding

Inputs reviewed in this pass:

- Repo rules from the previously read `AGENTS.md`: no file deletion, no new
  `rusqlite`, no bare `cass`, no bare `bv`, use `br --json`, and offload Rust
  build/test work through `rch`.
- Existing open swarm epic `coding_agent_session_search-oh96l` and children:
  contract, fixture harness, source adapters, status aggregator, stale-work
  engine, evidence broker, TUI surface, performance gate, docs, privacy
  guardrails, and integrated e2e freeze gate.
- Recent closed doctor-v2 beads, especially the proven patterns around
  archive-first safety, deterministic fixture factories, stable robot schemas,
  redaction reports, scripted e2e artifacts, and evidence-backed closeout.
- `bv --robot-plan`, which shows the current bottleneck is `.2`; it directly
  unlocks `.10` and `.11`, and the rest of the cockpit graph depends on those
  foundations.
- `bv --robot-insights`, which reports no graph cycles and identifies `.2` as
  the highest-value parallel cut for the current swarm cockpit graph.

Current blocker:

- `.beads` and the `.2` fixture paths are dirty and peer-owned by CreamRaven.
  Phase 5 bead creation is therefore recorded here as proposed `br` work, but
  not executed in this pass.

## Phase 2 - Thirty Ideas, Winnowed to Five

Initial thirty candidates:

1. Swarm replay lab for deterministic Agent Mail, Beads, git, rch, and process
   traces.
2. Recommendation decision journal that records what operators did and whether
   it worked.
3. Build pressure broker for `rch` slots, wrapper hangs, remote exits, and
   artifact retrieval health.
4. Declarative privacy policy compiler for all swarm evidence fields.
5. Saved `cass pack` recipes for common swarm incidents.
6. Stale-work time machine that reconstructs bead, mail, reservation, git, and
   process timelines.
7. Reservation conflict explainer with owner, path, dirty-file, and dependency
   context.
8. Evidence completeness score for each bead closeout and commit.
9. Large-host resource governor tuned for 64+ cores and 256GB+ RAM.
10. Quiet-mode swarm inspection that never touches live remotes or agents.
11. Cross-host source freshness map for remote cass mirrors.
12. TUI operator lane for compact swarm incident navigation.
13. Privacy-preserving evidence snippets with provenance hashes.
14. Fixture mutation fuzzer for provider outages and malformed snapshots.
15. Standard command transcript format for rch wrappers and tool runs.
16. Confidence calibration for stale-work and recommended-action outputs.
17. "What changed since last healthy swarm state" diff command.
18. Agent workload fairness view by bead family, locked files, and review load.
19. Operator runbook generator from actual incidents and proof artifacts.
20. Contract lint for robot JSON fields against docs and introspection schemas.
21. Swarm incident export bundle with redacted status, evidence, and commands.
22. Proof gap inbox that asks agents for missing verification artifacts.
23. Local-only simulation mode for upcoming multi-agent plans.
24. Build artifact retention budget and cleanup advisor.
25. Agent handoff packet validator for mail threads and bead close reasons.
26. Multi-run performance trend ledger for swarm aggregator sections.
27. Error taxonomy for provider degradation and partial swarm snapshots.
28. "Can I claim this?" explainer that combines br, reservations, dirty files,
   process state, and recent mail.
29. Schema evolution gate for swarm status goldens and fixture manifests.
30. Operator-level SLOs for swarm responsiveness and proof completeness.

Top five after winnowing:

1. **Swarm Replay Lab**

   A deterministic replay lab gives the project a way to test the cockpit
   against huge, hostile, and subtle swarm states without contacting live Agent
   Mail, running real `rch`, inspecting private sessions, or depending on a
   real dirty worktree. It extends the current `.2` fixture harness into a
   reusable simulation substrate. Users would experience this as trust: when
   the cockpit says a bead is stale, blocked, safe to claim, or under build
   pressure, that behavior has been replayed against known incidents and
   adversarial cases.

2. **Recommendation Decision Journal**

   The cockpit should not only recommend actions; it should learn whether its
   recommendations were useful. A local append-only journal can record the
   recommendation, confidence, evidence IDs, operator action, and eventual
   outcome. This is pragmatic because it reuses local robot surfaces and does
   not need network services. Over time it enables calibrated stale-work
   recommendations, fewer false positives, and better closeout proof.

3. **Build Pressure Broker**

   Large swarms waste time when agents cannot tell whether `rch` is genuinely
   running, remote-success-but-local-rsync is hanging, the machine is under
   formatting/test load, or a build slot is free. A broker can summarize local
   wrapper processes, remote exit evidence, Agent Mail build slots, and known
   target dirs into a branchable status. This directly addresses the current
   session pattern where remote tests passed but local artifact retrieval hung.

4. **Evidence Completeness Score**

   The existing evidence broker bead assembles proof. A completeness score
   makes missing proof visible and mechanical: changed paths, command shape,
   remote exit, stdout/stderr artifact, golden diff review, mail thread, bead
   close reason, and push/mirror state. This reduces "looks done" ambiguity and
   gives future agents a concrete next action instead of prose archaeology.

5. **Reservation Conflict Explainer**

   Agents need a safe answer to "can I claim this now?" The current workflow
   requires manual synthesis of `br`, Agent Mail reservations, dirty files,
   recent mail, and process liveness. A conflict explainer can return a compact
   JSON answer with blockers, owners, expiry, evidence, and suggested next
   coordination message. It is highly ergonomic and safety-preserving because it
   explains rather than force-releasing.

## Phase 3 - Next Ten Ideas

6. **Privacy Policy Compiler**

   Current and planned swarm surfaces need one redaction path. A small
   declarative policy plus generated tests would prevent drift across status,
   evidence, packs, docs, and future TUI views.

7. **Saved Pack Recipes for Swarm Incidents**

   Reusable `cass pack` recipes for "stalled bead", "missing proof", "build
   pressure", and "reservation conflict" would turn search history into fast,
   cited handoffs. This complements the evidence broker rather than replacing
   it.

8. **Fixture Mutation Fuzzer**

   The deterministic fixture harness should be stress-tested by mutating
   provider snapshots: missing fields, stale timestamps, path-like secrets,
   malformed mail subjects, duplicate reservations, and truncated git output.

9. **Large-Host Resource Governor**

   A governor can define budgets for aggregation latency, output size, memory,
   process scans, and fixture generation at 10k+ bead scale. It complements the
   existing `.7` performance gate by making budgets actionable during runtime.

10. **Swarm Time Machine**

   Reconstructing "how did we get here?" from bead updates, mail, reservations,
   git commits, and rch command evidence would make takeover and incident
   review much easier.

11. **Quiet-Mode Inspection**

   Operators need a guaranteed no-contact mode for sensitive moments. This mode
   would read only local cached snapshots and return explicit provider gaps.

12. **Command Transcript Standard**

   Standard JSONL transcripts for `rch`, `br`, `bv`, git, and cass robot
   commands would make evidence ingestion and failure reproduction predictable.

13. **Handoff Packet Validator**

   Before closing a bead, agents could validate that the closeout contains bead
   ID, commit, changed paths, tests, remote exit status, mail context, and known
   residual risk.

14. **SLO Dashboard for Swarm Responsiveness**

   Local SLOs such as "status under 100ms on cached fixtures" or "proof gap
   detection under 250ms" would convert performance intent into contract.

15. **Operator Runbook Generator**

   Once incidents are journaled and replayed, cass can generate runbooks from
   actual local outcomes instead of generic docs.

## Phase 4 - Overlap Check

Direct overlaps with current `oh96l` beads:

- `.2` already owns deterministic fixture/golden harness basics.
- `.3` already owns the first read-only status aggregator.
- `.4` already owns stale-work recommendation basics.
- `.5` already owns the first evidence broker.
- `.7` already owns large-swarm performance proof.
- `.11` already owns privacy and redaction guardrails.

Therefore this plan should not duplicate those first-wave deliverables. The
new work should be a second-wave extension that depends on the current cockpit
graph after `.2`, `.3`, `.4`, `.5`, `.7`, and `.11` land.

Merge decisions:

- Replay Lab extends `.2` rather than replacing it.
- Decision Journal extends `.3`, `.4`, and `.5`.
- Build Pressure Broker extends `.3`, `.5`, and `.7`.
- Evidence Completeness Score extends `.5`.
- Reservation Conflict Explainer extends `.3`, `.4`, and `.11`.
- Privacy Policy Compiler should be part of `.11` if `.11` has not started,
  otherwise a follow-up child of `.11`.

## Phase 5 - Proposed Bead Graph

Blocked in this pass:

- Do not run `br create`, `br update`, or `br dep add` while `.beads` is dirty
  from the active `.2` lane.
- When `.beads` is clean, create these with `br` only and immediately validate
  with `bv --robot-insights`.

Proposed epic:

```bash
br create "Epic: Swarm replay lab, recommendation journal, and build-pressure broker" \
  -p 1 -t epic --body "<self-contained body from this section>"
```

Proposed children:

1. **Define swarm replay trace schema and privacy boundaries**

   Background: Replay needs stable fixture contracts before generators or
   journals exist.

   Scope: Define JSON schema for provider snapshots, event timelines,
   redaction summary, command transcripts, and expected status outputs.

   Tests: Golden schema tests, hostile path fixtures, unknown-field policy,
   and round-trip fixture load/save coverage.

   Dependencies: current `oh96l.2`, `oh96l.11`.

2. **Implement replay trace loader and deterministic runner**

   Scope: Load trace bundles and feed existing fixtureable adapters without
   contacting live providers or mutating state.

   Tests: Healthy, dirty-peer, missing-provider, stale-clock, and conflicting
   reservation traces with stdout/stderr artifact capture.

   Dependencies: trace schema, current `oh96l.10`, current `oh96l.3`.

3. **Build synthetic large-swarm trace generator**

   Scope: Generate 10k bead, 500 reservation, 200 commit, 100 agent, and noisy
   process snapshots with deterministic seeds and privacy-safe fake values.

   Tests: Seed stability, size budget, distribution checks, malformed snapshot
   variants, and ignored stress gate.

   Dependencies: trace schema, current `oh96l.7`.

4. **Add recommendation decision journal schema**

   Scope: Append-only local records for recommendation ID, evidence IDs,
   confidence, selected action, operator notes, and eventual outcome.

   Tests: Redacted serialization, corruption recovery, no raw session content,
   and stable robot schema.

   Dependencies: current `oh96l.3`, `oh96l.5`, `oh96l.11`.

5. **Calibrate recommendation confidence from journal outcomes**

   Scope: Compare recommended actions with outcomes and emit calibration
   summaries without hidden automation or external services.

   Tests: False-positive stale cases, true stale cases, manual-review outcomes,
   and clock-skew scenarios.

   Dependencies: decision journal, current `oh96l.4`.

6. **Implement build-pressure broker over rch and process evidence**

   Scope: Classify idle, running, high pressure, remote-success-local-retrieval,
   stuck wrapper, missing proof, and unknown states from local snapshots and
   optional evidence ledgers.

   Tests: Fixture cases for remote exit 0 with local rsync hang, active
   rustfmt, multiple target dirs, stale process samples, and Agent Mail build
   slot conflicts.

   Dependencies: current `oh96l.3`, `oh96l.5`, `oh96l.7`.

7. **Add evidence completeness scoring for beads and commits**

   Scope: Score proof coverage across changed paths, tests, rch command shape,
   remote exit, artifacts, golden review, mail thread, close reason, and push
   state.

   Tests: Complete proof, missing command, interrupted artifact retrieval,
   conflicting proof, unrelated dirty files, and docs-only exceptions.

   Dependencies: current `oh96l.5`, `oh96l.11`.

8. **Add reservation conflict explainer**

   Scope: Provide a robot answer for claimability with owner, path, expiry,
   dirty-file evidence, recent mail, process liveness, and recommended next
   coordination message.

   Tests: Active owner, expired reservation, dirty peer work, explicit handoff,
   and false-positive stale prevention.

   Dependencies: current `oh96l.3`, `oh96l.4`, `oh96l.11`.

9. **Add saved pack recipes for swarm incidents**

   Scope: Provide named, documented pack/search recipes for stalled work,
   missing proof, build pressure, conflict handoff, and post-commit review.

   Tests: Robot-doc goldens, recipe schema tests, no bare `cass`, no private
   raw content, and examples using `--robot` or `--json`.

   Dependencies: current `oh96l.5`, current `oh96l.8`.

10. **Fixture mutation fuzzer for swarm providers**

    Scope: Mutate replay traces and provider snapshots to catch panic,
    redaction, determinism, and degraded-provider bugs.

    Tests: Fuzz corpus seeds, minimized repro artifacts, and no live provider
    access.

    Dependencies: replay runner, trace generator.

11. **Integrated replay e2e and golden freeze gate**

    Scope: Run replay lab, journal, broker, conflict explainer, and evidence
    score through cross-surface e2e fixtures.

    Tests: Command transcripts, stdout/stderr, parsed JSON, timing summary,
    redaction report, assertion summary, and reviewed goldens.

    Dependencies: all implementation children above.

12. **Docs and operator runbook for replay and calibration**

    Scope: README, robot-docs, and planning docs explaining how replay,
    decision journaling, build pressure, evidence scoring, and conflict
    explanation relate to Beads, Agent Mail, `rch`, and `cass pack`.

    Tests: Golden robot-doc checks and docs safety invariants.

    Dependencies: integrated replay e2e, current `oh96l.8`.

## Phase 6 - Plan-Space Refinement Passes

Pass 1 - Deduplication:

- Removed first-wave status, TUI, privacy, evidence, and performance tasks that
  are already represented by `oh96l` children.
- Reframed this plan as a second-wave extension after the current cockpit
  graph lands.

Pass 2 - Dependency sanity:

- Every proposed implementation task depends on the current cockpit foundation
  instead of bypassing it.
- The replay schema is the first new blocker; replay runner and generator
  build on it; integrated e2e waits for all implementation tasks.

Pass 3 - Testing posture:

- Added conformance-style schema tests for replay traces.
- Added golden tests for replay outputs, pack recipes, docs, and integrated
  e2e.
- Added fuzz/mutation coverage for provider snapshots and redaction failures.
- Added performance stress as ignored artifacts rather than routine CI load.

Pass 4 - Safety and privacy:

- Explicitly banned live provider contact in replay tests.
- Required redaction summaries and hostile path fixtures before any evidence
  output is trusted.
- Kept all takeover and reservation behavior advisory by default.

Pass 5 - Operator value:

- Prioritized features that reduce current observed friction: blocked queue
  detection, remote-success/local-wrapper ambiguity, proof gaps, claimability,
  and duplicated manual synthesis across `br`, Agent Mail, git, process, and
  `rch` state.
- Deferred nice-to-have UI work until robot contracts and replay gates prove
  the behavior.

## Creation Checklist For Later

When the active `.2` lane has committed or released `.beads`:

1. Re-run `git status --short --branch` and confirm `.beads` is clean.
2. Re-run `br ready --json`, `br list --status in_progress --json`, and
   `bv --robot-plan`.
3. Create the epic and children above with `br create`.
4. Add dependencies with `br dep add`.
5. Validate `bv --robot-insights` reports no cycles.
6. Commit only the `.beads` changes from that bead-creation pass.
