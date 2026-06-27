import { RecordId, Table, type Surreal } from "surrealdb"
import type { EnrollmentToken, Job, JobCreate, Group, GroupCreate, Client, Execution } from "@/lib/model"

/* ── helpers ── */

function rid(id: string): RecordId {
  const sep = id.indexOf(":")
  if (sep === -1) throw new Error(`Invalid record ID: ${id}`)
  return new RecordId(id.slice(0, sep), id.slice(sep + 1))
}

/* ── Jobs ── */

export async function getJobs(client: Surreal): Promise<Job[]> {
  const result = await client.select<Job>(new Table("job"))
  return result as unknown as Job[]
}

export async function getJobById(
  client: Surreal,
  id: string
): Promise<Job | null> {
  const result = await client.select<Job>(rid(id))
  return (result as unknown as Job) ?? null
}

export async function createJob(
  client: Surreal,
  data: JobCreate
): Promise<Job> {
  const result = await client.create<Job>(new Table("job")).content(data)
  if (!result) {
    throw new Error("Failed to create job")
  } else {
    return (result as unknown as Job)
  }
}

export async function updateJob(
  client: Surreal,
  id: string,
  data: Partial<JobCreate>
): Promise<Job> {
  const recordId = rid(id)
  const keys = Object.keys(data)
  if (keys.length === 0) throw new Error("No fields to update")

  // Use SET instead of MERGE to replace object fields entirely
  // (MERGE recursively merges object-typed enums like `enabled`)
  const assignments = keys.map(k => `\`${k}\` = $${k}`).join(", ")
  const [raw] = await client.query<Job[]>(
    `UPDATE ${recordId} SET ${assignments} RETURN AFTER`,
    data
  ).collect()

  const job = (raw as any)?.result?.[0] ?? raw
  if (!job) {
    throw new Error(`Failed to update job: ${id}`)
  }
  return job as unknown as Job
}

export async function deleteJob(
  client: Surreal,
  id: string
): Promise<void> {
  await client.delete(rid(id))
}

/* ── Groups ── */

export async function getGroups(client: Surreal): Promise<Group[]> {
  const result = await client.select<Group>(new Table("group"))
  return result as unknown as Group[]
}

export async function getGroupById(
  client: Surreal,
  id: string
): Promise<Group> {
  const result = await client.select<Group>(rid(id))
  if (!result) {
    console.error("[api] getGroupById failed:", id)
    throw new Error(`Group not found: ${id}`)
  } else {
    return (result as unknown as Group)
  }
}

export async function createGroup(
  client: Surreal,
  data: GroupCreate
): Promise<Group> {
  const result = await client.create<Group>(new Table("group")).content(data)
  if (!result) {
    throw new Error("Failed to create group")
  } else {
    return (result as unknown as Group)
  }
}

export async function updateGroup(
  client: Surreal,
  id: string,
  data: Partial<GroupCreate>
): Promise<Group> {
  console.log("updateGroup", { id, data })
  const result = await client.update<Group>(rid(id)).merge(data)
  return result as unknown as Group
}

export async function deleteGroup(
  client: Surreal,
  id: string
): Promise<void> {
  await client.delete(rid(id))
}

/* ── Clients ── */

export async function getClients(client: Surreal): Promise<Client[]> {
  const result = await client.select<Client>(new Table("client"))
  return result as unknown as Client[]
}

export async function getClientById(
  client: Surreal,
  id: string
): Promise<Client> {
  const result = await client.select<Client>(rid(id))
  if (!result) {
    console.error("[api] getClientById failed:", id)
    throw new Error(`Client not found: ${id}`)
  } else {
    return (result as unknown as Client)
  }
}

/* ── Executions ── */

export async function getExecutionsForJob(
  client: Surreal,
  jobId: RecordId
): Promise<Execution[]> {
  const [raw] = await client.query<Execution[]>(
    "SELECT * FROM execution WHERE job_id = $job_id ORDER BY created_at DESC",
    { job_id: jobId }
  ).collect()
  return (raw as any)?.result ?? []
}

export async function getExecutionById(
  client: Surreal,
  id: string
): Promise<Execution | null> {
  const result = await client.select<Execution>(rid(id))
  return (result as unknown as Execution) ?? null
}

/* ── Enrollment Tokens ── */

export async function getEnrollmentTokens(client: Surreal): Promise<EnrollmentToken[]> {
  const [raw] = await client.query<EnrollmentToken[]>(
    "SELECT * FROM enrollment_token ORDER BY created_at DESC"
  ).collect()
  return (raw as any)?.result ?? []
}

export async function getEnrollmentTokenById(
  client: Surreal,
  id: string
): Promise<EnrollmentToken | null> {
  const result = await client.select<EnrollmentToken>(rid(id))
  return (result as unknown as EnrollmentToken) ?? null
}

export interface CreateEnrollmentTokenResult {
  record: EnrollmentToken
  plaintext_token: string
}

export async function createEnrollmentToken(
  client: Surreal,
  opts: { single_use: boolean; expires_at: string | null }
): Promise<CreateEnrollmentTokenResult> {
  const plaintext_token = generateToken()
  const params: Record<string, unknown> = {
    raw_token: plaintext_token,
    single_use: opts.single_use,
  }
  if (opts.expires_at) {
    params.expires_at = opts.expires_at
  }
  const queryResult = await client.query<EnrollmentToken[]>(
    `CREATE enrollment_token CONTENT {
      token_hash: crypto::sha256($raw_token),
      valid: true,
      single_use: $single_use,
      expires_at: $expires_at ?? NONE,
      issued_by: $auth.id
    }`,
    params
  ).collect()

  const first = (queryResult as any)?.[0]
  const record = first as EnrollmentToken | undefined

  if (!record) {
    throw new Error(
      `Failed to create enrollment token.\n` +
      `queryResult type: ${typeof queryResult}, isArray: ${Array.isArray(queryResult)}\n` +
      `queryResult: ${JSON.stringify(queryResult)}\n` +
      `first type: ${typeof first}, isArray: ${Array.isArray(first)}\n` +
      `first: ${JSON.stringify(first)}`
    )
  }
  return { record, plaintext_token }
}

export async function deleteEnrollmentToken(
  client: Surreal,
  id: string
): Promise<void> {
  await client.delete(rid(id))
}

function generateToken(): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
  let result = ""
  const array = new Uint8Array(32)
  crypto.getRandomValues(array)
  for (let i = 0; i < 32; i++) {
    const byte = array[i] ?? 0
    result += chars[byte % chars.length]
  }
  return result
}

/* ── Helpers ── */

export function getJobsForClient(clientId: RecordId, jobs: Job[], groups: Group[]): Job[] {
  console.log("getJobsForClient", { clientId, jobs, groups })
  const clientGroupIds = groups
    .filter(g => g.members.some(m => String(m) === String(clientId)))
    .map(g => g.id)

  return jobs.filter(job =>
    job.assignments.some(a =>
      String(a) === String(clientId) ||
      clientGroupIds.some(gid => String(gid) === String(a))
    )
  )
}

export function getGroupsForClient(clientId: RecordId, groups: Group[]): Group[] {
  return groups.filter(group =>
    group.members.some(m => String(m) === String(clientId))
  )
}

export function getJobsForGroup(groupId: RecordId, jobs: Job[]): Job[] {
  return jobs.filter(job =>
    job.assignments.some(a => String(a) === String(groupId))
  )
}

export async function getClientsForGroup(client: Surreal, group: Group): Promise<Client[]> {
  console.log("getClientsForGroup", { group })
  console.log("group.members", group.members)
  const cids = group.members.map(m => String(m));
  const clients = await client.query<Client[]>("SELECT * FROM client WHERE id IN $cids", { cids }).collect()
  if (!clients) {
    throw new Error("Failed to get clients for group")
  } else {
    return clients
  }
}
