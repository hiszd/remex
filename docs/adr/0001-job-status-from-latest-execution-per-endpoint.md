# ADR 0001: Compute Job Status from Latest Execution Per Endpoint

## Status
Accepted

## Context
A Job can be instant, scheduled-once, or recurring. Previously, the `execution_status` computed field on the Job table aggregated *all* Executions ever tied to that job, regardless of time or endpoint.

For recurring jobs, this meant a single historical failure (e.g., an Endpoint was offline one day) would mark the entire Job as `Failed` forever, even if all subsequent runs succeeded. The system lacked a unified way to determine a Job's current health.

## Decision
Compute `execution_status` by looking at the **most recent Execution for each Endpoint** targeted by the Job. The status is then determined by aggregating across those "latest-per-endpoint" results:

- **Pending** — no Endpoint has produced an Execution yet
- **Completed** — the latest Execution on every targeted Endpoint is `Completed`
- **Failed** — the latest Execution on any targeted Endpoint is `Failed`
- **TimedOut** — the latest Execution on all targeted Endpoints is `TimedOut`
- **Running** — none of the above

This treats instant, scheduled-once, and recurring jobs identically. An instant job only runs once, so its "latest" is its only Execution — same logic applies.

## Consequences
- Historical executions remain in the database for audit and diagnostics, but do not influence the Job's current status.
- A future UI or API feature is needed to visualize historical execution trends (e.g., "show all executions for Job X on Endpoint Y").
- The `execution_status` computed field in SurrealDB must be updated to reflect this new logic.
