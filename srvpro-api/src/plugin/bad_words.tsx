// SRVPRO_PLUGIN_NAME srvpro::plugin::bad_words
import { useEffect, useState } from 'preact/hooks'
import { App as AntdApp, Button, Form, Input, Select, Space, Switch, Tabs } from 'antd'
import type {} from '@srvpro/portal-sdk'

type Group = {
  strategy: string
  words: string[]
}

type Row = {
  key: string
  strategy: string
  words: string
}

const STRATEGIES = [['block', '屏蔽'], ['warn', '警告'], ['echo', '回显'], ['kick', '踢出']]

function parse_groups(value: string): Row[] {
  try {
    const parsed = JSON.parse(value) as Record<string, Partial<Group>>
    return Object.entries(parsed).map(([key, group]) => ({ key, strategy: group.strategy ?? 'warn', words: (group.words ?? []).join('\n') }))
  } catch {
    return []
  }
}

async function read_configs(): Promise<Record<string, string>> {
  const response = await srvpro_fetch('/api/config?prefix=bad_words')
  if (!response.ok) throw new Error(`无法读取配置：${response.status}`)
  return await response.json() as Record<string, string>
}

async function post(path: string, body: unknown): Promise<void> {
  const response = await srvpro_fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
}

export function BadWords({ enabled }: { enabled?: boolean }) {
  const [check_room_name, set_check_room_name] = useState(false)
  const [check_player_name, set_check_player_name] = useState(false)
  const [max_length, set_max_length] = useState('')
  const [length_strategy, set_length_strategy] = useState('warn')
  const [rows, set_rows] = useState<Row[]>([])
  const [active, set_active] = useState(0)
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  useEffect(() => {
    read_configs().then((values) => {
      set_check_room_name(values['bad_words_check_room_name'] === 'true')
      set_check_player_name(values['bad_words_check_player_name'] === 'true')
      set_max_length(values['bad_words_max_length'] ?? '')
      set_length_strategy(values['bad_words_length_strategy'] ?? 'warn')
      set_rows(parse_groups(values['bad_words_groups'] ?? ''))
    }).catch((failure) => message.error(failure instanceof Error ? failure.message : '无法读取配置'))
  }, [])

  const update = (index: number, patch: Partial<Row>) => set_rows((current) => current.map((row, position) => (position === index ? { ...row, ...patch } : row)))

  const add = () => {
    set_active(rows.length)
    set_rows((current) => [...current, { key: '', strategy: 'warn', words: '' }])
  }

  const remove = (index: number) => {
    set_active((current) => (current > index ? current - 1 : current >= index ? Math.max(index - 1, 0) : current))
    set_rows((current) => current.filter((_, position) => position !== index))
  }

  const save = async () => {
    set_pending(true)
    try {
      await post('/api/config', { key: 'bad_words_check_room_name', value: String(check_room_name) })
      await post('/api/config', { key: 'bad_words_check_player_name', value: String(check_player_name) })
      await post('/api/config', { key: 'bad_words_max_length', value: max_length })
      await post('/api/config', { key: 'bad_words_length_strategy', value: length_strategy })
      const groups: Record<string, Group> = {}
      for (const row of rows) {
        if (row.key === '') continue
        groups[row.key] = { strategy: row.strategy, words: row.words.split('\n').map((word) => word.trim()).filter((word) => word !== '') }
      }
      await post('/api/bad_words', { groups })
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      set_pending(false)
    }
  }

  return (
    <>
      <Form layout="vertical" disabled={enabled === false} style={{ width: 720 }}>
        <Space size={64} align="start">
          <Form.Item label="检查房间名"><Switch checked={check_room_name} onChange={set_check_room_name} /></Form.Item>
          <Form.Item label="检查玩家名"><Switch checked={check_player_name} onChange={set_check_player_name} /></Form.Item>
        </Space>
        <Space size={24} align="start">
          <Form.Item label="最大长度"><Input value={max_length} placeholder="0" onChange={(event: Event) => set_max_length((event.target as HTMLInputElement).value)} style={{ width: 200 }} /></Form.Item>
          <Form.Item label="超长处理"><Select value={length_strategy} options={STRATEGIES.map(([value, label]) => ({ value, label }))} onChange={set_length_strategy} style={{ width: 160 }} /></Form.Item>
        </Space>
        <Tabs
          type="editable-card"
          hideAdd={enabled === false}
          activeKey={rows.length === 0 ? undefined : String(Math.min(active, rows.length - 1))}
          onChange={(key) => set_active(Number(key))}
          onEdit={(key, action) => {
            if (enabled === false) return
            action === 'add' ? add() : remove(Number(key))
          }}
          items={rows.map((row, index) => ({
            key: String(index),
            closable: enabled !== false,
            label: row.key === '' ? `组 ${index + 1}` : row.key,
            children: (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
                <Space align="start">
                  <Form.Item label="组名" style={{ marginBottom: 0 }}><Input value={row.key} placeholder="组名" onChange={(event: Event) => update(index, { key: (event.target as HTMLInputElement).value })} style={{ width: 240 }} /></Form.Item>
                  <Form.Item label="策略" style={{ marginBottom: 0 }}><Select value={row.strategy} options={STRATEGIES.map(([value, label]) => ({ value, label }))} onChange={(strategy: string) => update(index, { strategy })} style={{ width: 120 }} /></Form.Item>
                </Space>
                <Input.TextArea rows={10} value={row.words} placeholder="每行一条" onChange={(event: Event) => update(index, { words: (event.target as HTMLTextAreaElement).value })} style={{ width: '100%' }} />
              </div>
            ),
          }))}
        />
      </Form>
      <Button type="primary" loading={pending} onClick={save} style={{ marginTop: 16 }}>保存</Button>
    </>
  )
}
