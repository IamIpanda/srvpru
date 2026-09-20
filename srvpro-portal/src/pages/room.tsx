import { useEffect, useState } from 'preact/hooks'
import { NotificationOutlined, PauseOutlined, PlayCircleOutlined, RedoOutlined, ReloadOutlined, SettingOutlined, StopOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Card, Empty, Input, Modal, Space, Table, Tag, Tooltip } from 'antd'
import type { TableColumnsType } from 'antd'
import { configs, shout, start, stop, stopState } from '../api'
import { SettingsModal } from '../components/settings-modal'
import { applyRoomEvent, watchRooms } from '../roomlist'
import type { Room, RoomPlayer } from '../roomlist'

const DEFAULT_SERVER_NAME = 'Srvpru Server'

function contestants(room: Room): RoomPlayer[] {
  return room.users.filter((user) => user.pos < 7).sort((left, right) => left.pos - right.pos)
}

function playersOf(room: Room): string {
  const players = contestants(room)
  if (players.length === 0) return '-'
  if (players.length === 1) return `${players[0].name}（等待对手中）`
  return players.map((user) => user.name).join(' vs ')
}

function scoreOf(room: Room): string {
  if (room.istart !== 'start') return '-'
  const players = contestants(room)
  return `${players[0]?.status?.score ?? 0} : ${players[1]?.status?.score ?? 0}`
}

type Command = 'shout' | 'stop'

type CommandSpec = {
  title: string
  danger: boolean
  placeholder: string
  run: (text: string) => Promise<void>
}

const commands: Record<Command, CommandSpec> = {
  shout: { title: '广播', danger: false, placeholder: '广播给所有房间的内容', run: shout },
  stop: { title: '停服', danger: true, placeholder: '停服提示，玩家加入时可见', run: stop },
}

const roomColumns: TableColumnsType<Room> = [
  {
    title: '房间名',
    dataIndex: 'roomname',
    render: (name: string, room) => (
      <Space size={4}>
        {name}
        {room.needpass === 'true' && <Tag color="orange">密码</Tag>}
      </Space>
    ),
  },
  { title: '玩家', render: (_, room) => playersOf(room) },
  { title: '比分', width: 100, render: (_, room) => scoreOf(room) },
  { title: '操作', width: 160, render: () => null },
]

export function Room() {
  const [rooms, setRooms] = useState<Room[]>([])
  const [revision, setRevision] = useState(0)
  const [command, setCommand] = useState<Command>()
  const [draft, setDraft] = useState('')
  const [pending, setPending] = useState(false)
  const [stopped, setStopped] = useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [serverName, setServerName] = useState(DEFAULT_SERVER_NAME)
  const { message } = AntdApp.useApp()

  const spec = command ? commands[command] : undefined

  const refreshStopState = async () => {
    try {
      setStopped((await stopState()).stopped)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法获取停服状态')
    }
  }

  useEffect(() => {
    document.title = serverName
  }, [serverName])

  useEffect(() => {
    configs('api_name').then((entries) => setServerName(entries.api_name ?? DEFAULT_SERVER_NAME)).catch(() => {})
  }, [])

  useEffect(() => {
    setRooms([])
    refreshStopState()
    return watchRooms({
      onEvent: (event) => setRooms((current) => applyRoomEvent(current, event)),
    })
  }, [revision])

  const close = () => {
    setCommand(undefined)
    setDraft('')
  }

  const submit = async () => {
    if (spec === undefined || command === undefined) return
    setPending(true)
    try {
      await spec.run(draft)
      if (command === 'stop') setStopped(true)
      message.success(`${spec.title}成功`)
      close()
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : `${spec.title}失败`)
    } finally {
      setPending(false)
    }
  }

  const openServer = async () => {
    setPending(true)
    try {
      await start()
      setStopped(false)
      message.success('开服成功')
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '开服失败')
    } finally {
      setPending(false)
    }
  }

  return (
    <div class="room">
      <Card
        title={serverName}
        extra={
          <Space size={4}>
            <Tooltip title="刷新"><Button type="text" icon={<ReloadOutlined />} onClick={() => setRevision((value) => value + 1)} /></Tooltip>
            <Tooltip title="广播"><Button type="text" icon={<NotificationOutlined />} onClick={() => setCommand('shout')} /></Tooltip>
            <Tooltip title="禁止"><Button type="text" icon={<StopOutlined />} /></Tooltip>
            {stopped
              ? <Tooltip title="开服"><Button type="text" danger loading={pending} icon={<PlayCircleOutlined />} onClick={openServer} /></Tooltip>
              : <Tooltip title="停服"><Button type="text" danger icon={<PauseOutlined />} onClick={() => setCommand('stop')} /></Tooltip>}
            <Tooltip title="重启"><Button type="text" danger icon={<RedoOutlined />} /></Tooltip>
            <Tooltip title="设置"><Button type="text" icon={<SettingOutlined />} onClick={() => setSettingsOpen(true)} /></Tooltip>
          </Space>
        }
      >
        <Table columns={roomColumns} dataSource={rooms} rowKey="roomname" pagination={false} size="middle" locale={{ emptyText: <Empty /> }} />
      </Card>
      <Modal
        open={spec !== undefined}
        title={spec?.title ?? ''}
        okText="确定"
        okButtonProps={{ danger: spec?.danger ?? false, disabled: draft.trim() === '' }}
        confirmLoading={pending}
        closeIcon={null}
        footer={(_, { OkBtn }) => <OkBtn />}
        onOk={submit}
        onCancel={close}
      >
        <Input
          autoFocus
          placeholder={spec?.placeholder ?? ''}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onPressEnter={submit}
        />
      </Modal>
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  )
}
