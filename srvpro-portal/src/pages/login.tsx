import { useState } from 'preact/hooks'
import { route } from 'preact-router'
import { Alert } from 'antd'
import { Button } from 'antd'
import { Card } from 'antd'
import { Form } from 'antd'
import { Input } from 'antd'
import { Typography } from 'antd'
import { login } from '../auth'

type Credentials = {
  username: string
  password: string
}

export function Login() {
  const [error, setError] = useState<string>()
  const [pending, setPending] = useState(false)

  const submit = async (credentials: Credentials) => {
    setPending(true)
    setError(undefined)
    try {
      await login(credentials.username, credentials.password)
      route('/')
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : '登录失败')
    } finally {
      setPending(false)
    }
  }

  return (
    <div class="login">
      <Card style={{ width: 360 }}>
        <Typography.Title level={3} style={{ marginTop: 0 }}>srvpro</Typography.Title>
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
