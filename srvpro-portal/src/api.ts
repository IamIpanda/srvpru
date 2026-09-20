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

export function shout(text: string): Promise<void> {
  return get('/api/shout', { shout: text })
}

export function stop(text: string): Promise<void> {
  return get('/api/stop', { stop: text })
}

export function start(): Promise<void> {
  return get('/api/stop', { stop: 'false' })
}

export function stopState(): Promise<StopState> {
  return get('/api/getstop', {})
}

export function configs(prefix: string): Promise<Record<string, string>> {
  return get('/api/config', { prefix })
}

export function setConfig(key: string, value: string): Promise<void> {
  return send('POST', '/api/config', { key, value })
}

export function deleteConfig(key: string): Promise<void> {
  return send('DELETE', '/api/config', { key })
}

export function listUsers(): Promise<User[]> {
  return get<{ users: User[] }>('/api/users', {}).then((result) => result.users)
}

export function saveUser(user: UserDraft): Promise<void> {
  return send('POST', '/api/users', user)
}

export function deleteUser(username: string): Promise<void> {
  return send('DELETE', '/api/users', { username })
}

export function listPlugins(): Promise<string[]> {
  return get<{ plugins: string[] }>('/api/plugins', {}).then((result) => result.plugins)
}

export function enablePlugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/enable', { plugins })
}

export function disablePlugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/disable', { plugins })
}

export function refreshPlugins(plugins: string[]): Promise<void> {
  return send('POST', '/api/plugins/refresh', { plugins })
}

export function saveBadWords(groups: unknown): Promise<void> {
  return send('POST', '/api/bad_words', { groups })
}

function get<Result>(path: string, parameters: Record<string, string>): Promise<Result> {
  const query = new URLSearchParams(parameters).toString()
  return fetchJson(query ? `${path}?${query}` : path)
}

function send<Result>(method: 'POST' | 'DELETE', path: string, body: unknown): Promise<Result> {
  return fetchJson(path, { method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
}

async function fetchJson<Result>(path: string, init: { method?: string; headers?: Record<string, string>; body?: string } = {}): Promise<Result> {
  const credential = token()
  const response = await fetch(path, {
    method: init.method,
    headers: { ...init.headers, ...(credential ? { authorization: `Bearer ${credential}` } : {}) },
    body: init.body,
  })
  if (response.status === 403) {
    expire()
    throw new Error('权限不足或登录已过期')
  }
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
  return (await response.json()) as Result
}
