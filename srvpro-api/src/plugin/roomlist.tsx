// SRVPRO_PLUGIN_NAME srvpro_api::plugin::roomlist
import { ReloadOutlined } from '@ant-design/icons'
import { Button, Tooltip } from 'antd'

export type PlayerStatus = {
  score: number
  lp: number
  cards?: number | null
}

export type RoomPlayer = {
  id: string
  name: string
  ip?: string | null
  status?: PlayerStatus | null
  pos: number
}

export type Room = {
  roomid?: string | null
  roomname: string
  roommode?: number | null
  needpass: string
  users: RoomPlayer[]
  istart: string
  start_time?: number | null
}

export type RoomEvent =
  | { event: 'init'; data: Room[] }
  | { event: 'create'; data: Room }
  | { event: 'update'; data: Room }
  | { event: 'delete'; data: { name: string; started: boolean } }

const RETRY_DELAY = 2000
const refresh_handlers = new Set<() => void>()

export function on_refresh(handler: () => void): () => void {
  refresh_handlers.add(handler)
  return () => { refresh_handlers.delete(handler) }
}

export function apply_room_event(rooms: Room[], event: RoomEvent): Room[] {
  if (event.event === 'init') return event.data
  if (event.event === 'delete') return rooms.filter((room) => room.roomname !== event.data.name)
  const room = event.data
  if (!rooms.some((current) => current.roomname === room.roomname)) return [...rooms, room]
  return rooms.map((current) => (current.roomname === room.roomname ? room : current))
}

export function watch_rooms(on_event: (event: RoomEvent) => void): () => void {
  let socket: WebSocket | undefined
  let retry: number | undefined
  let stopped = false

  const open = () => {
    const credential = localStorage.getItem('srvpro_token')
    const query = credential === null ? '' : `?pass=${encodeURIComponent(credential)}`
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    socket = new WebSocket(`${protocol}//${location.host}/roomlist/ws${query}`)
    socket.onmessage = (message) => on_event(JSON.parse(message.data as string) as RoomEvent)
    socket.onclose = () => { if (!stopped) retry = window.setTimeout(open, RETRY_DELAY) }
  }

  open()

  return () => {
    stopped = true
    window.clearTimeout(retry)
    socket?.close()
  }
}

export function Refresh() {
  return <Tooltip title="刷新"><Button type="text" icon={<ReloadOutlined />} onClick={() => { for (const handler of refresh_handlers) handler() }} /></Tooltip>
}

