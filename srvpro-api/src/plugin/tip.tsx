// SRVPRO_PLUGIN_NAME srvpro::plugin::tip
import { useEffect, useState } from 'preact/hooks'
import { App as AntdApp, Button, Form, Input } from 'antd'
import type {} from '@srvpro/portal-sdk'

const INTERVAL_KEY = 'tip_interval'
const PREFIX_KEY = 'tip_prefix'
const TIPS_KEY = 'tip_tips'

function parse_tips(value: string): string[] {
  try {
    const parsed: unknown = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.map((tip) => String(tip)) : []
  } catch {
    return []
  }
}

async function read_configs(): Promise<Record<string, string>> {
  const response = await srvpro_fetch('/api/config?prefix=tip')
  if (!response.ok) throw new Error(`无法读取配置：${response.status}`)
  return await response.json() as Record<string, string>
}

async function post(path: string, body: unknown): Promise<void> {
  const response = await srvpro_fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
}

export function Tip({ enabled }: { enabled?: boolean }) {
  const [interval, set_interval] = useState('')
  const [prefix, set_prefix] = useState('')
  const [tips, set_tips] = useState('')
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  useEffect(() => {
    read_configs().then((values) => {
      set_interval(values[INTERVAL_KEY] ?? '')
      set_prefix(values[PREFIX_KEY] ?? '')
      set_tips(parse_tips(values[TIPS_KEY] ?? '').join('\n'))
    }).catch((failure) => message.error(failure instanceof Error ? failure.message : '无法读取配置'))
  }, [])

  const save = async () => {
    set_pending(true)
    try {
      await post('/api/config', { key: INTERVAL_KEY, value: interval })
      await post('/api/config', { key: PREFIX_KEY, value: prefix })
      await post('/api/tip', { tips: tips.split('\n').map((tip) => tip.trim()).filter((tip) => tip !== '') })
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      set_pending(false)
    }
  }

  return (
    <>
      <Form layout="vertical" disabled={enabled === false} style={{ width: 480 }}>
        <Form.Item label="间隔（毫秒）"><Input value={interval} placeholder="120000" onChange={(event: Event) => set_interval((event.target as HTMLInputElement).value)} /></Form.Item>
        <Form.Item label="前缀"><Input value={prefix} placeholder="Tip: " onChange={(event: Event) => set_prefix((event.target as HTMLInputElement).value)} /></Form.Item>
        <Form.Item label="提示列表"><Input.TextArea rows={8} value={tips} placeholder="每行一条" onChange={(event: Event) => set_tips((event.target as HTMLTextAreaElement).value)} /></Form.Item>
      </Form>
      <Button type="primary" loading={pending} onClick={save}>保存</Button>
    </>
  )
}
