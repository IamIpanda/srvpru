// SRVPRO_PLUGIN_NAME srvpro_api::auth
import { useState } from 'preact/hooks'
import { route } from 'preact-router'
import { Alert, Button, Card, Form, Input, Typography } from 'antd'

const TOKEN_KEY = 'srvpro_token'
const LOGIN_PATH = '/api/login'
const original_fetch = globalThis.fetch

async function fetch_with_authentication(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const url = new URL(input instanceof Request ? input.url : String(input), location.href)
  const same_origin = url.origin === location.origin
  const headers = new Headers(init?.headers ?? (input instanceof Request ? input.headers : undefined))
  const credential = localStorage.getItem(TOKEN_KEY)
  if (same_origin && credential !== null) headers.set('authorization', `Bearer ${credential}`)
  const response = await original_fetch(input, { ...init, headers })
  if (same_origin && response.status === 401 && url.pathname !== LOGIN_PATH) {
    localStorage.removeItem(TOKEN_KEY)
    route('/login')
  }
  return response
}

globalThis.srvpro_fetch = fetch_with_authentication

type Credentials = {
  username: string
  password: string
}

async function login(credentials: Credentials): Promise<void> {
  const response = await fetch('/api/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(credentials) })
  if (!response.ok) throw new Error(response.status === 403 ? '用户名或密码错误' : `登录失败：${response.status}`)
  const result = await response.json() as { token: string }
  localStorage.setItem(TOKEN_KEY, result.token)
}

export function Login() {
  const [error, set_error] = useState<string>()
  const [pending, set_pending] = useState(false)

  const submit = async (credentials: Credentials) => {
    set_pending(true)
    set_error(undefined)
    try {
      await login(credentials)
      route('/')
    } catch (failure) {
      set_error(failure instanceof Error ? failure.message : '登录失败')
    } finally {
      set_pending(false)
    }
  }

  return (
    <div class="login">
      <Card style={{ width: 360 }}>
        <Typography.Title level={3} style={{ marginTop: 0 }}>Srvpru</Typography.Title>
        {error && <Alert type="error" message={error} showIcon style={{ marginBottom: 16 }} />}
        <Form<Credentials> layout="vertical" onFinish={submit}>
          <Form.Item name="username" label="用户名" rules={[{ required: true, message: '请输入用户名' }]}>
            <Input autoComplete="username" />
          </Form.Item>
          <Form.Item name="password" label="密码" rules={[{ required: true, message: '请输入密码' }]}>
            <Input.Password autoComplete="current-password" />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={pending} block>登录</Button>
        </Form>
      </Card>
    </div>
  )
}
