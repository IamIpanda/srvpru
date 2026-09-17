const TOKEN_KEY = 'srvpro_token'

export function token(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export async function login(username: string, password: string): Promise<void> {
  const response = await fetch('/api/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  if (!response.ok) {
    throw new Error(response.status === 403 ? '用户名或密码错误' : `登录失败：${response.status}`)
  }
  const result = (await response.json()) as { token: string }
  localStorage.setItem(TOKEN_KEY, result.token)
}
