// SRVPRO_PLUGIN_NAME srvpro::plugin::stop
import { useEffect, useState } from 'preact/hooks'
import type { ComponentType } from 'preact'
import { PauseOutlined, PlayCircleOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Input, Modal, Tooltip } from 'antd'
import type {} from '@srvpro/portal-sdk'

const TITLE = '停服'

type StopState = {
  stopped: boolean
}

export function Stop() {
  const [stopped, set_stopped] = useState(false)
  const [opening, set_opening] = useState(false)
  const [open, set_open] = useState(false)
  const [draft, set_draft] = useState('')
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  const refresh = async () => {
    const response = await srvpro_fetch('/api/getstop')
    if (!response.ok) return
    set_stopped((await response.json() as StopState).stopped)
  }

  useEffect(() => { refresh().catch(() => undefined) }, [])

  const close = () => { set_open(false); set_draft('') }

  const open_server = async () => {
    set_opening(true)
    try {
      const response = await srvpro_fetch('/api/stop?stop=false')
      if (!response.ok) throw new Error(`开服失败：${response.status}`)
      message.success('开服成功')
      await refresh()
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : '开服失败')
    } finally {
      set_opening(false)
    }
  }

  const stop_server = async () => {
    set_pending(true)
    try {
      const response = await srvpro_fetch(`/api/stop?stop=${encodeURIComponent(draft)}`)
      if (!response.ok) throw new Error(`停服失败：${response.status}`)
      message.success(`${TITLE}成功`)
      close()
      await refresh()
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : `${TITLE}失败`)
    } finally {
      set_pending(false)
    }
  }

  return (
    <>
      {stopped
        ? <Tooltip title="开服"><Button type="text" danger loading={opening} icon={<PlayCircleOutlined />} onClick={open_server} /></Tooltip>
        : <Tooltip title={TITLE}><Button type="text" danger icon={<PauseOutlined />} onClick={() => set_open(true)} /></Tooltip>}
      {open && (
        <Modal
          open
          title={TITLE}
          okText="确定"
          okButtonProps={{ danger: true, disabled: draft.trim() === '' }}
          confirmLoading={pending}
          closeIcon={null}
          footer={(_: unknown, { OkBtn }: { OkBtn: ComponentType }) => <OkBtn />}
          onOk={stop_server}
          onCancel={close}
        >
          <Input autoFocus value={draft} placeholder="停服提示，玩家加入时可见" onChange={(event: Event) => set_draft((event.target as HTMLInputElement).value)} onPressEnter={stop_server} />
        </Modal>
      )}
    </>
  )
}
