// SRVPRO_PLUGIN_NAME srvpro_api::plugin::user
import { useEffect, useState } from 'preact/hooks'
import { DeleteOutlined, EditOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Input, Popconfirm, Select, Space, Table, Tag, Tooltip, type TableColumnsType } from 'antd'
import type {} from '@srvpro/portal-sdk'

type User = {
  username: string
  permissions: string[]
}

type Draft = User & {
  password: string
}

const PERMISSIONS: string[] = ['all', 'stop', 'shout', 'get_rooms', 'change_settings', 'manage_users', 'start_death', 'duel_log']

async function load_users(): Promise<User[]> {
  const response = await srvpro_fetch('/api/users')
  if (!response.ok) throw new Error(`无法读取用户：${response.status}`)
  return (await response.json() as { users: User[] }).users
}

async function send(method: 'POST' | 'DELETE', body: unknown): Promise<void> {
  const response = await srvpro_fetch('/api/users', { method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
}

export function Users() {
  const [users, set_users] = useState<User[]>([])
  const [draft, set_draft] = useState<Draft>({ username: '', password: '', permissions: [] })
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  const refresh = async () => set_users(await load_users())

  useEffect(() => { refresh().catch((failure) => message.error(failure instanceof Error ? failure.message : '无法读取用户')) }, [])

  const save = async () => {
    set_pending(true)
    try {
      await send('POST', draft)
      set_draft({ username: '', password: '', permissions: [] })
      await refresh()
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      set_pending(false)
    }
  }

  const remove = async (username: string) => {
    try {
      await send('DELETE', { username })
      await refresh()
      message.success('删除成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '删除失败')
    }
  }

  const columns: TableColumnsType<User> = [
    { title: '用户名', dataIndex: 'username' },
    { title: '权限', dataIndex: 'permissions', render: (values: string[]) => values.map((value) => <Tag key={value}>{value}</Tag>) },
    {
      title: '操作',
      width: 110,
      render: (_, user) => (
        <Space size={4}>
          <Tooltip title="编辑"><Button type="text" icon={<EditOutlined />} onClick={() => set_draft({ username: user.username, password: '', permissions: user.permissions })} /></Tooltip>
          <Popconfirm title="删除该用户？" onConfirm={() => remove(user.username)}><Tooltip title="删除"><Button type="text" danger icon={<DeleteOutlined />} /></Tooltip></Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <Space orientation="vertical" size={12} style={{ width: 840 }}>
      <Space wrap>
        <Input placeholder="用户名" value={draft.username} onChange={(event: Event) => set_draft({ ...draft, username: (event.target as HTMLInputElement).value })} style={{ width: 160 }} />
        <Input.Password placeholder="密码（留空则不改）" value={draft.password} onChange={(event: Event) => set_draft({ ...draft, password: (event.target as HTMLInputElement).value })} style={{ width: 200 }} />
        <Select mode="multiple" placeholder="权限" value={draft.permissions} options={PERMISSIONS.map((permission) => ({ value: permission, label: permission }))} onChange={(values: string[]) => set_draft({ ...draft, permissions: values })} style={{ minWidth: 240 }} />
        <Button type="primary" loading={pending} disabled={draft.username === ''} onClick={save}>保存</Button>
      </Space>
      <Table columns={columns} dataSource={users} rowKey="username" pagination={false} size="small" />
    </Space>
  )
}
