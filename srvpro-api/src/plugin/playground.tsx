// SRVPRO_PLUGIN_NAME srvpro::core
import { useEffect, useState } from 'preact/hooks'
import { BugOutlined, DownloadOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Dropdown, Empty, Input, Modal, Space, Table, Tag, Tooltip, type TableColumnsType } from 'antd'
import type { PortalManifest } from '@srvpro/portal-sdk'
import { count_duel_logs, load_duel_logs } from './duel_log'
import type { DuelLog } from './duel_log'
import { apply_room_event, on_refresh, watch_rooms } from './roomlist'
import type { Room, RoomPlayer } from './roomlist'

const ROOMLIST = 'srvpro_api::plugin::roomlist'
const DUEL_LOG = 'srvpro_api::plugin::duel_log'
const PAGE_SIZE = 20

type Row = {
  key: string
  name: string
  player_a: string
  player_b: string
  score: string
  time: string
  needpass: boolean
  room?: string
  replay?: string
  duel_count?: number
}

function contestants(room: Room): RoomPlayer[] {
  return room.users.filter((user) => user.pos < 7).sort((left, right) => left.pos - right.pos)
}

function room_players(room: Room): [string, string] {
  const players = contestants(room)
  if (players.length === 0) return ['', '']
  if (players.length === 1) return [players[0].name, '（等待对手中）']
  return [players[0].name, players[1].name]
}

function score_of(room: Room): string {
  if (room.istart !== 'start') return '-'
  const players = contestants(room)
  return `${players[0]?.status?.score ?? 0}:${players[1]?.status?.score ?? 0}`
}

function format_time(time: string): string {
  const [start, end] = time.split(' - ')
  if (start === undefined || end === undefined) return time.slice(0, 16)
  return `${start.slice(0, 16)} - ${end.slice(0, 5)}`
}

function format_timestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

function matches_keyword(room: Room, keyword: string): boolean {
  if (keyword === '') return true
  const needle = keyword.toLowerCase()
  return [room.roomname, ...room_players(room)].some((text) => text.toLowerCase().includes(needle))
}

function room_row(room: Room): Row {
  const [player_a, player_b] = room_players(room)
  return { key: room.roomname, name: room.roomname, player_a, player_b, score: score_of(room), time: room.start_time === undefined || room.start_time === null ? '-' : `${format_timestamp(room.start_time)} - `, needpass: room.needpass === 'true', room: room.roomname }
}

function log_row(log: DuelLog): Row {
  const names = log.players.map((player) => player.real_name)
  return { key: `log-${log.id}`, name: log.name, player_a: names[0] ?? '', player_b: names[1] ?? '', score: log.players.map((player) => player.score).join(':'), time: format_time(log.time), needpass: false, replay: log.id, duel_count: log.duel_count }
}

async function read_manifest(): Promise<PortalManifest | null> {
  const response = await fetch('/api/manifest')
  return response.ok ? await response.json() as PortalManifest : null
}

async function load_field(room: string): Promise<unknown> {
  const response = await srvpro_fetch(`/api/field/${encodeURIComponent(room)}`)
  if (!response.ok) throw new Error(`无法获取房间 field：${response.status}`)
  return response.json()
}

async function download_replay(id: string, index: number, filename: string): Promise<void> {
  const response = await srvpro_fetch(`/api/replay/${encodeURIComponent(id)}/${index}`)
  if (!response.ok) throw new Error(`无法下载录像：${response.status}`)
  const element = document.createElement('a')
  element.href = URL.createObjectURL(await response.blob())
  element.download = filename
  element.style.display = 'none'
  document.body.appendChild(element)
  element.click()
  document.body.removeChild(element)
}

function Highlight({ text, keyword }: { text: string; keyword: string }) {
  const index = keyword === '' ? -1 : text.toLowerCase().indexOf(keyword.toLowerCase())
  if (index < 0) return <>{text}</>
  return <>{text.slice(0, index)}<span class="highlight">{text.slice(index, index + keyword.length)}</span>{text.slice(index + keyword.length)}</>
}

export function Playground() {
  const [rooms, set_rooms] = useState<Room[]>([])
  const [logs, set_logs] = useState<DuelLog[]>([])
  const [logs_total, set_logs_total] = useState(0)
  const [draft, set_draft] = useState('')
  const [keyword, set_keyword] = useState('')
  const [loading, set_loading] = useState(false)
  const [page, set_page] = useState(1)
  const [field, set_field] = useState<{ room: string; text: string } | null>(null)
  const [generation, set_generation] = useState(0)
  const { message } = AntdApp.useApp()

  const visible_rooms = rooms.filter((room) => matches_keyword(room, keyword))
  const page_start = (page - 1) * PAGE_SIZE
  const room_rows = visible_rooms.slice(page_start, page_start + PAGE_SIZE)
  const limit = PAGE_SIZE - room_rows.length
  const offset = Math.max(0, page_start - visible_rooms.length)

  useEffect(() => on_refresh(() => set_generation((value) => value + 1)), [])

  useEffect(() => {
    let dispose: () => void = () => undefined
    let cancelled = false

    const start = async () => {
      const manifest = await read_manifest()
      if (cancelled || manifest === null) return
      const enabled = new Set(manifest.pages.map((page) => page.plugin))
      if (enabled.has(ROOMLIST)) {
        set_rooms([])
        dispose = watch_rooms((event) => {
          set_rooms((current) => apply_room_event(current, event))
          if (event.event === 'delete' && event.data.started && enabled.has(DUEL_LOG)) set_generation((value) => value + 1)
        })
      }
    }

    start().catch(() => undefined)

    return () => { cancelled = true; dispose() }
  }, [generation])

  useEffect(() => {
    if (limit <= 0) { set_logs([]); return }
    set_loading(true)
    load_duel_logs({ keyword, limit, offset }).then(set_logs).catch(() => set_logs([])).finally(() => set_loading(false))
  }, [keyword, limit, offset, generation])

  useEffect(() => {
    count_duel_logs(keyword).then(set_logs_total).catch(() => set_logs_total(0))
  }, [keyword, generation])

  const show_field = async (room: string) => {
    try {
      set_field({ room, text: JSON.stringify(await load_field(room), null, 2) })
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法获取房间 field')
    }
  }

  const save_replay = async (row: Row, index: number) => {
    try {
      await download_replay(row.replay ?? '', index, `${row.player_a}_vs_${row.player_b}_G${index + 1}.yrp`)
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '无法下载录像')
    }
  }

  const columns: TableColumnsType<Row> = [
    { title: '时间', width: 220, dataIndex: 'time' },
    { title: '房间', dataIndex: 'name', render: (name: string, row) => <Space size={4}><Highlight text={name} keyword={keyword} />{row.needpass && <Tag color="orange">密码</Tag>}</Space> },
    { title: '玩家A', dataIndex: 'player_a', align: 'end', render: (player: string) => <Highlight text={player} keyword={keyword} /> },
    { title: '比分', width: 60, align: 'center', dataIndex: 'score' },
    { title: '玩家B', dataIndex: 'player_b', render: (player: string) => <Highlight text={player} keyword={keyword} /> },
    { title: '操作', width: 80, align: 'center', render: (_, row) => row.room === undefined
      ? <Tooltip title="下载录像"><Dropdown trigger={['hover']} disabled={(row.duel_count ?? 0) === 0} menu={{ items: Array.from({ length: row.duel_count ?? 0 }, (_, index) => ({ key: String(index), label: `G${index + 1}`, onClick: () => save_replay(row, index) })) }}><Button type="text" icon={<DownloadOutlined />} /></Dropdown></Tooltip>
      : <Tooltip title="输出 field"><Button type="text" icon={<BugOutlined />} onClick={() => show_field(row.room ?? '')} /></Tooltip> },
  ]

  const data = [...room_rows.map(room_row), ...logs.map(log_row)]

  return (
    <>
      <Input allowClear placeholder="搜索房名 / 玩家..." value={draft} onChange={(event: Event) => set_draft((event.target as HTMLInputElement).value)} onPressEnter={() => { set_keyword(draft); set_page(1) }} style={{ marginBottom: 12 }} />
      <Table columns={columns} dataSource={data} rowKey="key" loading={loading} pagination={{ align: 'center', showSizeChanger: false, pageSize: PAGE_SIZE, total: visible_rooms.length + logs_total, onChange: set_page }} locale={{ emptyText: <Empty /> }} />
      {field !== null && (
        <Modal open title={`field：${field.room}`} width={720} footer={null} onCancel={() => set_field(null)}>
          <pre style={{ margin: 0, maxHeight: '60vh', overflow: 'auto' }}>{field.text}</pre>
        </Modal>
      )}
    </>
  )
}

