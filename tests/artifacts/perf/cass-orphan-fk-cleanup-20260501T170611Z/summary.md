# Orphan FK Cleanup Profiling Pass

Date: 2026-05-01

Scope: indexer startup `cleanup_orphan_fk_rows` on the live cass DB, measured without committing repair deletes to the live DB.

## Baseline

- `cass index --watch-once /tmp/cass-fk-probe-nonexistent --json` timed out after 122s in `preparing`.
- Before and after counts stayed unchanged because the timeout rolled back the repair transaction:
  - `orphan_messages`: 3583
  - `orphan_msg_child_metrics`: 3583
  - `orphan_msg_child_token_usage`: 3583
- Frame-pointer perf sample showed the narrowed workload under `run_index -> query_row_with_params`, walking the `messages` btree during FK orphan probing.

## Rejected Query Shapes

- Old correlated message count:
  - `SELECT COUNT(*) FROM messages WHERE NOT EXISTS (...)`
  - `timeout 30`, rc 124, elapsed 31.20s, max RSS 10450184 KB.
- `GROUP BY conversation_id` discovery:
  - returned 51213 rows but still timed out at 30.36s with max RSS 17895948 KB.
- Aggregate message bounds:
  - `SELECT MIN(conversation_id), MAX(conversation_id) FROM messages`
  - `timeout 30`, rc 124, elapsed 30.92s, max RSS 10140068 KB.
- Open-ended outside-parent probes:
  - `conversation_id > 51214`: rc 0 but 27.32s and high RSS.
  - `conversation_id < 1`: rc 0 but 24.62s and high RSS.

## Accepted Query Shape

- Message conversation bounds via ordered index probes:
  - `ORDER BY conversation_id ASC LIMIT 1`: rc 0, elapsed 0.10s.
  - `ORDER BY conversation_id DESC LIMIT 1`: rc 0, elapsed 0.10s.
- Parent conversation range scan:
  - `SELECT id FROM conversations WHERE id BETWEEN 1 AND 51214 ORDER BY id`
  - rc 0, elapsed 0.40s, 51214 rows.
- Gap-point message lookups for the 34 missing parent IDs:
  - rc 0, elapsed 0.10s, returned 3583 orphan message IDs.

## Code Change

`cleanup_orphan_fk_rows` now:

1. Discovers message conversation-id bounds with ordered index probes.
2. Scans the much smaller parent `conversations` ID range.
3. Converts missing parent IDs into finite gap ranges.
4. Fetches orphan message IDs with bounded `conversation_id = ?` / `BETWEEN ? AND ?` probes.
5. Deletes dependent rows by chunked `message_id IN (...)` primary-key probes before deleting root orphan messages by ID.

This avoids the measured slow fsqlite shapes while preserving the existing report semantics: dependent rows below orphan messages are cleaned but not double-counted as root orphans.
