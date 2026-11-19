import type { ComponentChildren } from 'preact'
import { useEffect, useState } from 'preact/hooks'
import { SettingOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Empty, Form, Input, Modal, Select, Switch, Tabs, Tooltip } from 'antd'
import type { PortalConfigField, PortalConfigTab, PortalManifest } from '@srvpro/portal-sdk'
import { Slot } from './slot'

export function Settings({ tabs }: { tabs: PortalManifest['tabs'] }) {
  const [open, set_open] = useState(false)
  const items = tabs.map((tab) => ({ key: tab.plugin, label: tab.title, children: tab_content(tab) }))

  return (
    <>
      <Tooltip title="设置"><Button type="text" icon={<SettingOutlined />} onClick={() => set_open(true)} /></Tooltip>
      <Modal open={open} title="设置" footer={null} closable={false} width="fit-content" destroyOnHidden onCancel={() => set_open(false)}>
        {items.length === 0 ? <Empty /> : <Tabs tabPlacement="start" items={items} />}
      </Modal>
    </>
  )
}

function tab_content(tab: PortalConfigTab) {
  const content = (enabled?: boolean) => 'fields' in tab.content ? <ConfigFields plugin={tab.plugin} fields={tab.content.fields} enabled={enabled} /> : <Slot plugin={tab.plugin} name={tab.content.component.export} enabled={enabled} />
  if (!tab.toggle) return content(undefined)
  return <PluginToggle plugin={tab.plugin}>{content}</PluginToggle>
}

function PluginToggle({ plugin, children }: { plugin: string; children: (enabled: boolean | undefined) => ComponentChildren }) {
  const [enabled, set_enabled] = useState<boolean>()
  const { message } = AntdApp.useApp()

  useEffect(() => {
    read_plugins().then((plugins) => set_enabled(plugins.includes(plugin))).catch(() => undefined)
  }, [])

  const change = async (value: boolean) => {
    try {
      await post(`/api/plugins/${value ? 'enable' : 'disable'}`, { plugins: [plugin] })
      set_enabled(value)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '操作失败')
    }
  }

  return (
    <div>
      <Form.Item label="启用"><Switch checked={enabled === true} loading={enabled === undefined} onChange={change} /></Form.Item>
      {children(enabled)}
    </div>
  )
}

function ConfigFields({ plugin, fields, enabled }: { plugin: string; fields: PortalConfigField[]; enabled?: boolean }) {
  const [values, set_values] = useState<Record<string, string>>({})
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  useEffect(() => {
    read_configs().then(set_values).catch((failure) => message.error(failure instanceof Error ? failure.message : '无法读取配置'))
  }, [])

  const update = (key: string, value: string) => set_values((current) => ({ ...current, [key]: value }))

  const save = async () => {
    set_pending(true)
    try {
      for (const field of fields) await write_config(field.key, values[field.key] ?? '')
      if (enabled) await post('/api/plugins/refresh', { plugins: [plugin] })
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
        {fields.map((field) => <Form.Item key={field.key} label={field.label}>{control(field, values[field.key] ?? '', (value) => update(field.key, value))}</Form.Item>)}
      </Form>
      <Button type="primary" loading={pending} onClick={save}>保存</Button>
    </>
  )
}

function control(field: PortalConfigField, value: string, update: (value: string) => void) {
  if (field.kind === 'boolean') return <Switch checked={value === 'true'} onChange={(checked) => update(String(checked))} />
  if (field.kind === 'select') return <Select value={value === '' ? undefined : value} placeholder={field.default} options={field.options.map(([option, label]) => ({ value: option, label }))} onChange={update} style={{ minWidth: 160 }} />
  if (field.kind === 'tags') return <Select mode="tags" value={parse_tags(value)} placeholder={field.default} onChange={(tags) => update(JSON.stringify(tags))} style={{ width: '100%' }} />
  if (field.kind === 'multiline' || field.kind === 'json') return <Input.TextArea rows={field.kind === 'json' ? 8 : 4} value={value} placeholder={field.default} onChange={(event) => update(event.currentTarget.value)} />
  return <Input value={value} placeholder={field.default} onChange={(event) => update(event.currentTarget.value)} />
}

function parse_tags(value: string): string[] {
  try {
    const parsed: unknown = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.map((tag) => String(tag)) : []
  } catch {
    return []
  }
}

async function read_configs(): Promise<Record<string, string>> {
  const response = await srvpro_fetch('/api/config?prefix=')
  if (!response.ok) throw new Error(`无法读取配置：${response.status}`)
  return await response.json() as Record<string, string>
}

async function read_plugins(): Promise<string[]> {
  const response = await srvpro_fetch('/api/plugins')
  if (!response.ok) throw new Error(`无法读取插件：${response.status}`)
  return (await response.json() as { plugins: string[] }).plugins
}

function write_config(key: string, value: string): Promise<void> {
  return post('/api/config', { key, value })
}

async function post(path: string, body: unknown): Promise<void> {
  const response = await srvpro_fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  if (!response.ok) throw new Error(`请求失败：${response.status}`)
}
