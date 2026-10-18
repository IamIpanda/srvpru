import { useEffect, useState } from 'preact/hooks'
import { NotificationOutlined, PauseOutlined, PlayCircleOutlined, RedoOutlined, ReloadOutlined, SettingOutlined, StopOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Card, Empty, Input, Modal, Space, Table, Tag, Tooltip } from 'antd'
import type { TableColumnsType } from 'antd'
import { configs, list_duel_logs, shout, start, stop, stop_state } from '../api'
import type { DuelLog } from '../api'
import { SettingsModal } from '../components/settings-modal'
import { apply_room_event, watch_rooms } from '../roomlist'
import type { Room, RoomPlayer } from '../roomlist'

const DEFAULT_SERVER_NAME = 'Srvpru Server'

function contestants(room: Room): RoomPlayer[] {
  return room.users.filter((user) => user.pos < 7).sort((left, right) => left.pos - right.pos)
}

function players_of(room: Room): string {
  const players = contestants(room)
  if (players.length === 0) return '-'
  if (players.length === 1) return `${players[0].name}（等待对手中）`
  return players.map((user) => user.name).join(' vs ')
}

function score_of(room: Room): string {
  if (room.istart !== 'start') return '-'
  const players = contestants(room)
  return `${players[0]?.status?.score ?? 0} : ${players[1]?.status?.score ?? 0}`
}

type Row = {
  key: string
  name: string
  players: string
  score: string
  time: string
  needpass: boolean
}

function room_row(room: Room): Row {
  return {
    key: room.roomname,
    name: room.roomname,
    players: players_of(room),
    score: score_of(room),
    time: '-',
    needpass: room.needpass === 'true',
  }
}

function log_row(log: DuelLog): Row {
  return {
    key: `log-${log.id}`,
    name: log.name,
    players: log.players.map((player) => player.real_name).join(' vs '),
    score: log.players.map((player) => player.score).join(' : '),
    time: log.time,
    needpass: false,
  }
}

type TextDialogProps = {
  title: string
  placeholder: string
  danger?: boolean
  run: (text: string) => Promise<void>
  on_success?: () => void
  on_close: () => void
}

function TextDialog({ title, placeholder, danger = false, run, on_success, on_close }: TextDialogProps) {
  const [draft, set_draft] = useState('')
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  const submit = async () => {
    set_pending(true)
    try {
      await run(draft)
      on_success?.()
      message.success(`${title}成功`)
      on_close()
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : `${title}失败`)
    } finally {
      set_pending(false)
    }
  }

  return (
    <Modal
      open
      title={title}
      okText="确定"
      okButtonProps={{ danger, disabled: draft.trim() === '' }}
      confirmLoading={pending}
      closeIcon={null}
      footer={(_, { OkBtn }) => <OkBtn />}
      onOk={submit}
      onCancel={on_close}
    >
      <Input autoFocus value={draft} placeholder={placeholder} onChange={(event) => set_draft(event.currentTarget.value)} onPressEnter={submit} />
    </Modal>
  )
}

const columns: TableColumnsType<Row> = [
  {
    title: '名称',
    dataIndex: 'name',
    render: (name: string, row) => (
      <Space size={4}>
        {name}
        {row.needpass && <Tag color="orange">密码</Tag>}
      </Space>
    ),
  },
  { title: '玩家', dataIndex: 'players' },
  { title: '比分', width: 100, dataIndex: 'score' },
  { title: '时间', width: 180, dataIndex: 'time' },
  { title: '操作', width: 160, render: () => null },
]

export function Room() {
  const [rooms, set_rooms] = useState<Room[]>([])
  const [logs, set_logs] = useState<DuelLog[]>([])
  const [opening, set_opening] = useState(false)
  const [stopped, set_stopped] = useState(false)
  const [dialog, set_dialog] = useState('')
  const [server_name, set_server_name] = useState(DEFAULT_SERVER_NAME)
  const [_, fuck_react_hook_i_need_force_refresh] = useState(false)
  const { message } = AntdApp.useApp()

  const refresh_stop_state = async () => {
    try {
      set_stopped((await stop_state()).stopped)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法获取停服状态')
    }
  }

  useEffect(() => { document.title = server_name }, [server_name])
  useEffect(() => {
    configs('api_name').then((entries) => set_server_name(entries.api_name ?? DEFAULT_SERVER_NAME)).catch(() => {})
  }, [])

  useEffect(() => {
    set_rooms([])
    refresh_stop_state()
    list_duel_logs().then(set_logs).catch(() => set_logs([]))
    return watch_rooms({
      on_event: (event) => set_rooms((current) => apply_room_event(current, event)),
    })
  }, [fuck_react_hook_i_need_force_refresh])

  const close_dialog = () => set_dialog('')

  const open_server = async () => {
    set_opening(true)
    try {
      await start()
      set_stopped(false)
      message.success('开服成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '开服失败')
    } finally {
      set_opening(false)
    }
  }

  return (
    <div class="room">
      <Card
        title={server_name}
        extra={
          <Space size={4}>
            <Tooltip title="刷新"><Button type="text" icon={<ReloadOutlined />} onClick={() => fuck_react_hook_i_need_force_refresh((value) => !value)} /></Tooltip>
            <Tooltip title="广播"><Button type="text" icon={<NotificationOutlined />} onClick={() => set_dialog('shout')} /></Tooltip>
            <Tooltip title="禁止"><Button type="text" icon={<StopOutlined />} /></Tooltip>
            {stopped
              ? <Tooltip title="开服"><Button type="text" danger loading={opening} icon={<PlayCircleOutlined />} onClick={open_server} /></Tooltip>
              : <Tooltip title="停服"><Button type="text" danger icon={<PauseOutlined />} onClick={() => set_dialog('stop')} /></Tooltip>}
            <Tooltip title="重启"><Button type="text" danger icon={<RedoOutlined />} /></Tooltip>
            <Tooltip title="设置"><Button type="text" icon={<SettingOutlined />} onClick={() => set_dialog('settings')} /></Tooltip>
          </Space>
        }
      >
        <Table columns={columns} dataSource={[...rooms.map(room_row), ...logs.map(log_row)]} rowKey="key" pagination={{ pageSize: 10 }} size="middle" locale={{ emptyText: <Empty /> }} />
      </Card>
      {dialog === 'shout' && <TextDialog title="广播" placeholder="广播给所有房间的内容" run={shout} on_close={close_dialog} />}
      {dialog === 'stop' && <TextDialog danger title="停服" placeholder="停服提示，玩家加入时可见" run={stop} on_success={() => set_stopped(true)} on_close={close_dialog} />}
      {dialog === 'settings' && <SettingsModal open on_close={close_dialog} />}
    </div>
  )
}
