import { useEffect, useState } from 'preact/hooks'
import { DeleteOutlined, EditOutlined, ReloadOutlined, StopOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Form, Input, Modal, Popconfirm, Select, Space, Switch, Table, Tabs, Tag, theme, Tooltip } from 'antd'
import type { TableColumnsType } from 'antd'
import { configs, deleteConfig, deleteUser, disablePlugins, enablePlugins, listPlugins, listUsers, refreshPlugins, saveBadWords, saveUser, setConfig } from '../api'
import type { User } from '../api'

type SettingsModalProps = {
  open: boolean
  onClose: () => void
}

type GeneralField = {
  key: string
  label: string
  editable?: boolean
  fallback?: string
}

const generalFields: GeneralField[] = [
  { key: 'api_name', label: '服务器名称', editable: true },
  { key: 'auth_token_ttl_days', label: '登录有效期（天）', fallback: '180' },
  { key: 'api_port', label: 'HTTP 端口', fallback: '7922' },
  { key: 'server_port', label: 'ygopro 端口', fallback: '7911' },
]

const permissions = ['stop', 'shout', 'get_rooms', 'change_settings', 'manage_users', 'start_death']

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const [advanced, setAdvanced] = useState(false)

  const tabs = advanced
    ? [
        { key: 'plugins', label: '插件', children: <PluginsTab /> },
        { key: 'config', label: '值', children: <ConfigValuesTab /> },
      ]
    : [
        { key: 'general', label: '通用', children: <GeneralTab /> },
        { key: 'users', label: '用户', children: <UsersTab /> },
        { key: 'duel', label: '决斗', children: <DuelTab /> },
        { key: 'welcome', label: '欢迎语', children: <WelcomeTab /> },
        { key: 'tip', label: '小贴士', children: <TipTab /> },
        { key: 'dialogues', label: '登场台词', children: <DialoguesTab /> },
        { key: 'bad_words', label: '屏蔽词', children: <BadWordsTab /> },
        { key: 'report', label: '上报', children: <ReportTab /> },
      ]

  return (
    <Modal
      title={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', paddingBottom: 15 }}>
          设置
          <Switch checkedChildren="高级" unCheckedChildren="高级" checked={advanced} onChange={setAdvanced} />
        </div>
      }
      open={open}
      footer={null}
      closable={false}
      width="fit-content"
      destroyOnHidden
      onCancel={onClose}
    >
      <Tabs key={advanced ? 'advanced' : 'normal'} tabPlacement="start" items={tabs} />
    </Modal>
  )
}

function GeneralTab() {
  const [values, setValues] = useState<Record<string, string>>({})
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()

  useEffect(() => {
    Promise.all([configs('api'), configs('auth'), configs('server')]).then(([api, auth, server]) => setValues({ ...api, ...auth, ...server })).catch(() => {})
  }, [])

  const save = async () => {
    setPending(true)
    try {
      await setConfig('api_name', values.api_name ?? '')
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      setPending(false)
    }
  }

  return (
    <Form layout="vertical" style={{ width: 420 }}>
      {generalFields.map((field) => (
        <Form.Item key={field.key} label={field.label}>
          <Input
            disabled={!field.editable}
            value={values[field.key] ?? field.fallback ?? ''}
            onChange={(event) => setValues({ ...values, [field.key]: event.currentTarget.value })}
          />
        </Form.Item>
      ))}
      <Button type="primary" loading={pending} onClick={save}>保存</Button>
    </Form>
  )
}

function PluginsTab() {
  const [plugins, setPlugins] = useState<string[]>([])
  const [draft, setDraft] = useState('')
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()

  const load = async () => {
    try {
      setPlugins(await listPlugins())
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法读取插件')
    }
  }

  useEffect(() => {
    load()
  }, [])

  const run = async (action: () => Promise<void>, done: string) => {
    setPending(true)
    try {
      await action()
      await load()
      message.success(done)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : `${done}失败`)
    } finally {
      setPending(false)
    }
  }

  const columns: TableColumnsType<{ plugin: string }> = [
    { title: '插件', dataIndex: 'plugin' },
    {
      title: '操作',
      width: 90,
      align: 'center',
      render: (_, row) => (
        <Space size={4}>
          <Tooltip title="刷新配置">
            <Button type="text" size="small" icon={<ReloadOutlined />} onClick={() => run(() => refreshPlugins([row.plugin]), '刷新配置')} />
          </Tooltip>
          <Popconfirm title="禁用该插件？" onConfirm={() => run(() => disablePlugins([row.plugin]), '禁用')}>
            <Tooltip title="禁用">
              <Button type="text" danger size="small" icon={<StopOutlined />} />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <Space orientation="vertical" size={12} style={{ width: 640 }}>
      <Space wrap>
        <Input placeholder="插件名" value={draft} onChange={(event) => setDraft(event.currentTarget.value)} onPressEnter={() => draft !== '' && run(() => enablePlugins([draft]), '启用')} style={{ width: 360 }} />
        <Button type="primary" loading={pending} disabled={draft === ''} onClick={() => run(() => enablePlugins([draft]), '启用')}>启用</Button>
      </Space>
      <Table columns={columns} dataSource={plugins.map((plugin) => ({ plugin }))} rowKey="plugin" pagination={false} size="small" />
    </Space>
  )
}

type ConfigEntry = {
  key: string
  value: string
}

function ConfigValuesTab() {
  const [entries, setEntries] = useState<ConfigEntry[]>([])
  const [draftKey, setDraftKey] = useState('')
  const [draftValue, setDraftValue] = useState('')
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()

  const load = async () => {
    try {
      const result = await configs('')
      setEntries(Object.entries(result).map(([key, value]) => ({ key, value })).sort((left, right) => left.key.localeCompare(right.key)))
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法读取配置')
    }
  }

  useEffect(() => {
    load()
  }, [])

  const save = async () => {
    if (draftKey === '') return
    setPending(true)
    try {
      await setConfig(draftKey, draftValue)
      await load()
      setDraftKey('')
      setDraftValue('')
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      setPending(false)
    }
  }

  const remove = async (key: string) => {
    try {
      await deleteConfig(key)
      await load()
      message.success('删除成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '删除失败')
    }
  }

  const columns: TableColumnsType<ConfigEntry> = [
    { title: '键', dataIndex: 'key', width: 300, ellipsis: true },
    { title: '值', dataIndex: 'value', ellipsis: true },
    {
      title: '操作',
      width: 90,
      align: 'center',
      render: (_, entry) => (
        <Space size={4}>
          <Tooltip title="编辑">
            <Button type="text" size="small" icon={<EditOutlined />} onClick={() => { setDraftKey(entry.key); setDraftValue(entry.value) }} />
          </Tooltip>
          <Popconfirm title="删除该配置？" onConfirm={() => remove(entry.key)}>
            <Tooltip title="删除">
              <Button type="text" danger size="small" icon={<DeleteOutlined />} />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <Space orientation="vertical" size={12} style={{ width: 640 }}>
      <Space wrap>
        <Input placeholder="键" value={draftKey} onChange={(event) => setDraftKey(event.currentTarget.value)} style={{ width: 200 }} />
        <Input placeholder="值" value={draftValue} onChange={(event) => setDraftValue(event.currentTarget.value)} onPressEnter={save} style={{ width: 300 }} />
        <Button type="primary" loading={pending} disabled={draftKey === ''} onClick={save}>保存</Button>
      </Space>
      <Table columns={columns} dataSource={entries} rowKey="key" tableLayout="fixed" pagination={false} size="small" />
    </Space>
  )
}

function UsersTab() {
  const [users, setUsers] = useState<User[]>([])
  const [draft, setDraft] = useState({ username: '', password: '', permissions: [] as string[] })
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()

  const load = async () => {
    try {
      setUsers(await listUsers())
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法读取用户')
    }
  }

  useEffect(() => {
    load()
  }, [])

  const save = async () => {
    if (draft.username === '') return
    setPending(true)
    try {
      await saveUser(draft)
      setDraft({ username: '', password: '', permissions: [] })
      await load()
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      setPending(false)
    }
  }

  const remove = async (username: string) => {
    try {
      await deleteUser(username)
      await load()
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
      width: 90,
      align: 'center',
      render: (_, user) => (
        <Space size={4}>
          <Tooltip title="编辑">
            <Button type="text" size="small" icon={<EditOutlined />} onClick={() => setDraft({ username: user.username, password: '', permissions: user.permissions })} />
          </Tooltip>
          <Popconfirm title="删除该用户？" onConfirm={() => remove(user.username)}>
            <Tooltip title="删除">
              <Button type="text" danger size="small" icon={<DeleteOutlined />} />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <Space orientation="vertical" size={12} style={{ width: 840 }}>
      <Space wrap>
        <Input placeholder="用户名" value={draft.username} onChange={(event) => setDraft({ ...draft, username: event.currentTarget.value })} style={{ width: 160 }} />
        <Input.Password placeholder="密码（留空则不改）" value={draft.password} onChange={(event) => setDraft({ ...draft, password: event.currentTarget.value })} style={{ width: 200 }} />
        <Select
          mode="multiple"
          placeholder="权限"
          value={draft.permissions}
          options={permissions.map((permission) => ({ value: permission, label: permission }))}
          onChange={(values) => setDraft({ ...draft, permissions: values })}
          style={{ minWidth: 240 }}
        />
        <Button type="primary" loading={pending} onClick={save}>保存</Button>
      </Space>
      <Table columns={columns} dataSource={users} rowKey="username" pagination={false} size="small" />
    </Space>
  )
}

type FieldKind = 'text' | 'multi' | 'bool' | 'select' | 'json' | 'tags'

type Field = {
  key: string
  label: string
  kind: FieldKind
  options?: { value: string; label: string }[]
  fallback?: string
}

type Group = {
  title: string
  prefix: string
  plugin: string
  toggle?: boolean
  fields: Field[]
}

const welcomeGroups: Group[] = [
  {
    title: '欢迎语',
    prefix: 'welcome',
    plugin: 'srvpro::plugin::welcome',
    toggle: true,
    fields: [
      { key: 'welcome_message', label: '内容', kind: 'multi' },
    ],
  },
]

const duelGroups: Group[] = [
  {
    title: '随机匹配',
    prefix: 'random_match',
    plugin: 'srvpro::plugin::random_match',
    toggle: true,
    fields: [
      { key: 'random_match_default', label: '默认模式', kind: 'select', fallback: 'S', options: [{ value: 'S', label: 'S' }, { value: 'M', label: 'M' }, { value: 'T', label: 'T' }] },
      { key: 'random_match_modes', label: '模式列表', kind: 'tags', fallback: '[]' },
    ],
  },
  {
    title: '换备超时',
    prefix: 'side_timeout',
    plugin: 'srvpro::plugin::side_timeout',
    toggle: true,
    fields: [
      { key: 'side_timeout_side_timeout', label: '超时（分钟）', kind: 'text', fallback: '3' },
    ],
  },
  {
    title: '断线重连',
    prefix: 'reconnect',
    plugin: 'srvpro::plugin::reconnect',
    toggle: true,
    fields: [
      { key: 'reconnect_wait_time', label: '等待时间（秒）', kind: 'text', fallback: '60' },
      { key: 'reconnect_auto_surrender_after_disconnect', label: '掉线后自动投降', kind: 'bool', fallback: 'false' },
      { key: 'reconnect_allow_kick_reconnect', label: '允许踢人重连', kind: 'bool', fallback: 'false' },
    ],
  },
]

const tipGroups: Group[] = [
  {
    title: '定时提示',
    prefix: 'tip',
    plugin: 'srvpro::plugin::tip',
    toggle: true,
    fields: [
      { key: 'tip_enabled', label: '启用', kind: 'bool', fallback: 'true' },
      { key: 'tip_interval', label: '间隔（毫秒）', kind: 'text', fallback: '120000' },
      { key: 'tip_prefix', label: '前缀', kind: 'text', fallback: 'Tip: ' },
      { key: 'tip_tips', label: '提示列表', kind: 'json', fallback: '[]' },
    ],
  },
]

const dialoguesGroups: Group[] = [
  {
    title: '登场台词',
    prefix: 'dialogues',
    plugin: 'srvpro::plugin::dialogues',
    toggle: true,
    fields: [
      { key: 'dialogues_enabled', label: '启用', kind: 'bool', fallback: 'true' },
      { key: 'dialogues_dialogues', label: '台词（卡片密码 → 台词列表）', kind: 'json', fallback: '{}' },
    ],
  },
]

const reportGroups: Group[] = [
  {
    title: '对局上报',
    prefix: 'report',
    plugin: 'srvpro::plugin::report',
    toggle: true,
    fields: [
      { key: 'report_url', label: '地址', kind: 'text' },
      { key: 'report_access_key', label: '访问密钥', kind: 'text' },
    ],
  },
  {
    title: '卡组上报',
    prefix: 'deck_report',
    plugin: 'srvpro::plugin::deck_report',
    toggle: true,
    fields: [
      { key: 'deck_report_url', label: '地址', kind: 'text' },
      { key: 'deck_report_access_key', label: '访问密钥', kind: 'text' },
    ],
  },
]

function WelcomeTab() {
  return <ConfigTab groups={welcomeGroups} />
}

function DuelTab() {
  return <ConfigTab groups={duelGroups} />
}

function TipTab() {
  return <ConfigTab groups={tipGroups} />
}

function DialoguesTab() {
  return <ConfigTab groups={dialoguesGroups} />
}

const BAD_WORDS_PLUGIN = 'srvpro::plugin::bad_words'

function BadWordsTab() {
  const [checkRoomName, setCheckRoomName] = useState(true)
  const [checkPlayerName, setCheckPlayerName] = useState(true)
  const [groups, setGroups] = useState('{}')
  const [enabled, setEnabled] = useState(false)
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()
  const { token } = theme.useToken()

  useEffect(() => {
    configs('bad_words').then((result) => {
      setCheckRoomName((result.bad_words_check_room_name ?? 'true') !== 'false')
      setCheckPlayerName((result.bad_words_check_player_name ?? 'true') !== 'false')
      setGroups(result.bad_words_groups ?? '{}')
    }).catch(() => {})
    listPlugins().then((plugins) => setEnabled(plugins.includes(BAD_WORDS_PLUGIN))).catch(() => {})
  }, [])

  const togglePlugin = async (checked: boolean) => {
    setPending(true)
    try {
      if (checked) {
        await enablePlugins([BAD_WORDS_PLUGIN])
      } else {
        await disablePlugins([BAD_WORDS_PLUGIN])
      }
      setEnabled(checked)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '切换插件失败')
    } finally {
      setPending(false)
    }
  }

  const save = async () => {
    setPending(true)
    try {
      await setConfig('bad_words_check_room_name', String(checkRoomName))
      await setConfig('bad_words_check_player_name', String(checkPlayerName))
      await saveBadWords(JSON.parse(groups))
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      setPending(false)
    }
  }

  return (
    <Form layout="vertical" style={{ width: 420 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16, paddingBottom: 8, borderBottom: `1px solid ${token.colorSplit}`, fontWeight: 600 }}>
        屏蔽词
        <Switch checked={enabled} loading={pending} onChange={togglePlugin} />
      </div>
      <Form.Item label="检查房间名"><Switch disabled={!enabled} checked={checkRoomName} onChange={setCheckRoomName} /></Form.Item>
      <Form.Item label="检查玩家名"><Switch disabled={!enabled} checked={checkPlayerName} onChange={setCheckPlayerName} /></Form.Item>
      <Form.Item label="词库"><Input.TextArea disabled={!enabled} rows={10} value={groups} onChange={(event) => setGroups(event.currentTarget.value)} /></Form.Item>
      <Button type="primary" loading={pending} onClick={save}>保存</Button>
    </Form>
  )
}

function ReportTab() {
  return <ConfigTab groups={reportGroups} />
}

function parseTags(value: string): string[] {
  try {
    const parsed: unknown = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.map((tag) => String(tag)) : []
  } catch {
    return []
  }
}

function ConfigTab({ groups }: { groups: Group[] }) {
  const [values, setValues] = useState<Record<string, string>>({})
  const [enabledPlugins, setEnabledPlugins] = useState<string[]>([])
  const [pending, setPending] = useState(false)
  const { message } = AntdApp.useApp()
  const { token } = theme.useToken()

  useEffect(() => {
    const prefixes = [...new Set(groups.map((group) => group.prefix))]
    Promise.all(prefixes.map((prefix) => configs(prefix))).then((results) => setValues(Object.assign({}, ...results))).catch(() => {})
    if (groups.some((group) => group.toggle)) listPlugins().then(setEnabledPlugins).catch(() => {})
  }, [])

  const togglePlugin = async (plugin: string, enabled: boolean) => {
    setPending(true)
    try {
      if (enabled) {
        await enablePlugins([plugin])
      } else {
        await disablePlugins([plugin])
      }
      setEnabledPlugins(await listPlugins())
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '切换插件失败')
    } finally {
      setPending(false)
    }
  }

  const update = (key: string, value: string) => setValues((current) => ({ ...current, [key]: value }))

  const valueOf = (field: Field) => values[field.key] ?? field.fallback ?? ''

  const save = async () => {
    setPending(true)
    try {
      for (const group of groups) {
        for (const field of group.fields) {
          await setConfig(field.key, valueOf(field))
        }
      }
      await refreshPlugins(groups.map((group) => group.plugin))
      message.success('保存成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      setPending(false)
    }
  }

  const render = (group: Group, field: Field) => {
    const value = valueOf(field)
    const disabled = group.toggle === true && !enabledPlugins.includes(group.plugin)
    if (field.kind === 'bool') return <Switch disabled={disabled} checked={value === 'true'} onChange={(checked) => update(field.key, String(checked))} />
    if (field.kind === 'select') return <Select disabled={disabled} value={value} options={field.options} onChange={(selected) => update(field.key, selected)} style={{ width: 160 }} />
    if (field.kind === 'tags') return <Select mode="tags" open={false} disabled={disabled} value={parseTags(value)} onChange={(tags) => update(field.key, JSON.stringify(tags))} style={{ width: '100%' }} />
    if (field.kind === 'multi' || field.kind === 'json') return <Input.TextArea disabled={disabled} rows={4} value={value} onChange={(event) => update(field.key, event.currentTarget.value)} />
    return <Input disabled={disabled} value={value} onChange={(event) => update(field.key, event.currentTarget.value)} />
  }

  return (
    <Form layout="vertical" style={{ width: 420 }}>
      {groups.map((group, index) => (
        <div key={group.title}>
          {(groups.length > 1 || group.toggle) && (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: index === 0 ? 0 : 8, marginBottom: 16, paddingBottom: 8, borderBottom: `1px solid ${token.colorSplit}`, fontWeight: 600 }}>
              {group.title}
              {group.toggle && <Switch checked={enabledPlugins.includes(group.plugin)} loading={pending} onChange={(checked) => togglePlugin(group.plugin, checked)} />}
            </div>
          )}
          {group.fields.map((field) => (
            <Form.Item key={field.key} label={field.label}>{render(group, field)}</Form.Item>
          ))}
        </div>
      ))}
      <Button type="primary" loading={pending} onClick={save}>保存</Button>
    </Form>
  )
}
