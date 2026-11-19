// SRVPRO_PLUGIN_NAME srvpro_api::plugin::duel_log
import type {} from '@srvpro/portal-sdk'

export type DuelLogPlayer = {
  real_name: string
  ip: string
  score: number
}

export type DuelLog = {
  id: string
  time: string
  name: string
  duel_count: number
  replay_file_name: string
  players: DuelLogPlayer[]
}

export type DuelLogQuery = {
  keyword?: string
  limit?: number
  offset?: number
}

export async function load_duel_logs(query: DuelLogQuery = {}): Promise<DuelLog[]> {
  const parameters = new URLSearchParams()
  if (query.keyword !== undefined && query.keyword !== '') parameters.set('keyword', query.keyword)
  if (query.limit !== undefined) parameters.set('limit', String(query.limit))
  if (query.offset !== undefined) parameters.set('offset', String(query.offset))
  const response = await srvpro_fetch(`/api/duellog?${parameters}`)
  if (!response.ok) throw new Error(`无法读取对局记录：${response.status}`)
  return await response.json() as DuelLog[]
}

export async function count_duel_logs(keyword: string): Promise<number> {
  const parameters = new URLSearchParams()
  if (keyword !== '') parameters.set('keyword', keyword)
  const response = await srvpro_fetch(`/api/duellog/count?${parameters}`)
  if (!response.ok) throw new Error(`无法统计对局记录：${response.status}`)
  return (await response.json() as { total: number }).total
}

