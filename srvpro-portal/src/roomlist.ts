import { token } from './auth'

export type RoomConnection = 'connecting' | 'live' | 'offline'

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
}

export type RoomEvent =
  | { event: 'init'; data: Room[] }
  | { event: 'create'; data: Room }
  | { event: 'update'; data: Room }
  | { event: 'delete'; data: { name: string; started: boolean } }

export type RoomHandlers = {
  on_event: (event: RoomEvent) => void
  on_connection?: (connection: RoomConnection) => void
}

const RETRY_DELAY = 2000

export function apply_room_event(rooms: Room[], event: RoomEvent): Room[] {
  if (event.event === 'init') return event.data
  if (event.event === 'delete') return rooms.filter((room) => room.roomname !== event.data.name)
  const room = event.data
  if (!rooms.some((current) => current.roomname === room.roomname)) return [...rooms, room]
  return rooms.map((current) => (current.roomname === room.roomname ? room : current))
}

export function watch_rooms(handlers: RoomHandlers): () => void {
  let socket: WebSocket | undefined
  let retry: number | undefined
  let stopped = false

  const open = () => {
    handlers.on_connection?.('connecting')
    const query = new URLSearchParams()
    const credential = token()
    if (credential) query.set('pass', credential)
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    socket = new WebSocket(`${protocol}//${location.host}/roomlist/ws?${query}`)
    socket.onopen = () => handlers.on_connection?.('live')
    socket.onmessage = (message) => handlers.on_event(JSON.parse(message.data as string) as RoomEvent)
    socket.onclose = () => {
      if (stopped) return
      handlers.on_connection?.('offline')
      retry = window.setTimeout(open, RETRY_DELAY)
    }
  }

  open()

  return () => {
    stopped = true
    window.clearTimeout(retry)
    socket?.close()
  }
}
