import type { RecordId } from "surrealdb";

/* ── SurrealDB object-typed enums ── */

export type ExecutionStatus =
  | { Pending: Record<string, never> }
  | { Running: Record<string, never> }
  | { Completed: Record<string, never> }
  | { Failed: Record<string, never> }
  | { TimedOut: Record<string, never> }

export type Enabled =
  | { Draft: Record<string, never> }
  | { Enabled: Record<string, never> }
  | { Disabled: Record<string, never> }

export type JobType =
  | { Instant: Record<string, never> }
  | { Scheduled: string }
  | { Recurring: [string, number] }

export type ExecutionStatusVariant =
  | "Pending"
  | "Running"
  | "Completed"
  | "Failed"
  | "TimedOut"

export type EnabledVariant = "Draft" | "Enabled" | "Disabled"

/* ── Domain models ── */

export interface Job {
  id: RecordId
  job_name: string
  job_shell: string
  job_command: string
  job_type: JobType
  execution_status: ExecutionStatus
  enabled: Enabled
  assignments: RecordId[]
  created_at: string
  updated_at: string
}

export interface JobCreate {
  job_name: string
  job_shell: string
  job_command: string
  job_type: JobType
  enabled: Enabled
  assignments: RecordId[]
}

export interface Group {
  id: RecordId
  group_name: string
  members: RecordId[]
  created_at: string
  updated_at: string
}

export interface GroupCreate {
  group_name: string
  members: RecordId[]
}

export interface Client {
  id: RecordId
  client_name: string
  secret: string
  hardware_hash: string
  last_seen: string | null
  connection_history: Record<string, unknown>[]
  created_at: string
  updated_at: string
}

export interface Execution {
  id: RecordId
  job_id: RecordId | null
  client_id: RecordId
  status: ExecutionStatus
  output: string
  command: string
  exit_code: string
  execution_start: string | null
  execution_end: string | null
  created_at: string
  updated_at: string
}

export interface User {
  id: RecordId
  username: string
  email: string
  created_at: string
  updated_at: string
}

/* ── Enum helpers ── */

export function extractEnumVariant(obj: Record<string, unknown>): string {
  return Object.keys(obj)[0]
}

export function formatEnumVariant(variant: string): string {
  return variant.replace(/([a-z])([A-Z])/g, "$1 $2")
}

export function makeEnabled(variant: EnabledVariant): Enabled {
  return { [variant]: {} } as Enabled
}

export function makeExecutionStatus(
  variant: ExecutionStatusVariant
): ExecutionStatus {
  return { [variant]: {} } as ExecutionStatus
}

/* ── Human-readable field labels ── */

export const FIELD_LABELS: Record<string, string> = {
  job_name: "Name",
  job_shell: "Shell",
  job_command: "Command",
  job_type: "Type",
  execution_status: "Status",
  enabled: "State",
  assignments: "Assignments",
  group_name: "Name",
  members: "Members",
  client_name: "Client Name",
  hardware_hash: "Hardware Hash",
  last_seen: "Last Seen",
  created_at: "Created",
  updated_at: "Updated",
  output: "Output",
  command: "Command",
  exit_code: "Exit Code",
  execution_start: "Started",
  execution_end: "Ended",
  username: "Username",
  email: "Email",
}

export function fieldLabel(field: string): string {
  return FIELD_LABELS[field] ?? field
}
