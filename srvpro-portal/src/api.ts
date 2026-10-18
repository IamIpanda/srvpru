import { expire, token } from './auth'

export type StopState = {
  stopped: boolean
}

export type User = {
  username: string
  permissions: string[]
}

export type UserDraft = User & {
  password: string
}

export type DuelLogPlayer = {
  real_name: string
  ip: string | null
  is_first: boolean
  score: number
  lp: number | null
  card_count: number
  start_deck_buffer: string
  current_deck_buffer: string
  winner: boolean
}

export type DuelLog = {
  id: string
  time: string
  name: string
  room_id: number | null
  cloud_replay_id: string | null
  replay_file_name: string
  room_mode: number | null
  duel_count: number
  players: DuelLogPlayer[]
}

export function list_duel_logs(): Promise<DuelLog[]> {
  return get<DuelLog[]>('/api/duellog', {})
}

export function shout(text: string): Promise<void> {
  return get('/api/shout', { shout: text })
}

export function stop(text: string): Promise<void> {
  return get('/api/stop', { stop: text })
}

export function start(): Promise<void> {
  return get('/api/stop', { stop: 'false' })
}

export function stop_state(): Promise<StopState> {
  return get('/api/getstop', {})
}

export function configs(prefix: string): Promise<Record<string, string>> {
  return get('/api/config', { prefix })
}

export function set_config(key: string, value: string): Promise<void> {
  return send('POST', '/api/config', { key, value })
}

export function delete_config(key: string): Promise<void> {
  return send('DELETE', '/api/config', { key })
}

export function list_users(): Promise<User[]> {
  return get<{ users: User[] }>('/api/users', {}).then((result) => result.users)
}

export function save_user(user: UserDraft): Promise<void> {
  return send('POST', '/api/users', user)
}

export function delete_user(username: string): Promise<void> {
  return send('DELETE', '/api/users', { username })
}

export function list_plugins(): Promise<string[]> {
  return get<{ plugins: string[] }>('/api/plugins', {}).then((result) => result.plugins)
}

export function enable_plugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/enable', { plugins })
}

export function disable_plugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/disable', { plugins })
}

export function refresh_plugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/refresh', { plugins })
}

export function save_bad_words(groups: unknown): Promise<void> {
  return send('POST', '/api/bad_words', { groups })
}

function get<Result>(path: string, parameters: Record<string, string>): Promise<Result> {
  const query = new URLSearchParams(parameters).toString()
  return fetch_json(query ? `${path}?${query}` : path)
}

function send<Result>(method: 'POST' | 'DELETE', path: string, body: unknown): Promise<Result> {
  return fetch_json(path, { method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
}

async function fetch_json<Result>(path: string, init: { method?: string; headers?: Record<string, string>; body?: string } = {}): Promise<Result> {
  const credential = token()
  const response = await fetch(path, {
    method: init.method,
    headers: { ...init.headers, ...(credential ? { authorization: `Bearer ${credential}` } : {}) },
    body: init.body,
  })
  if (response.status === 401) {
    expire()
    throw new Error('登录已过期')
  }
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
  return (await response.json()) as Result
}
