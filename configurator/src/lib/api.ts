import { RecordId, Table, type Surreal } from "surrealdb"
import type { Job, JobCreate, Group, GroupCreate, Client } from "@/lib/model"

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
  return (result as unknown as Job[])[0]
}

export async function updateJob(
  client: Surreal,
  id: string,
  data: Partial<JobCreate>
): Promise<Job> {
  const result = await client.update<Job>(rid(id)).merge(data)
  return result as unknown as Job
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
): Promise<Group | null> {
  const result = await client.select<Group>(rid(id))
  return (result as unknown as Group) ?? null
}

export async function createGroup(
  client: Surreal,
  data: GroupCreate
): Promise<Group> {
  const result = await client.create<Group>(new Table("group")).content(data)
  return (result as unknown as Group[])[0]
}

export async function updateGroup(
  client: Surreal,
  id: string,
  data: Partial<GroupCreate>
): Promise<Group> {
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
): Promise<Client | null> {
  const result = await client.select<Client>(rid(id))
  return (result as unknown as Client) ?? null
}

/* ── Helpers ── */

export function getJobsForClient(clientId: RecordId, jobs: Job[]): Job[] {
  return jobs.filter(job => 
    job.assignments.some(a => String(a) === String(clientId))
  )
}

export function getGroupsForClient(clientId: RecordId, groups: Group[]): Group[] {
  return groups.filter(group => 
    group.members.some(m => String(m) === String(clientId))
  )
}
