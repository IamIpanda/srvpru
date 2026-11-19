// SRVPRO_PLUGIN_NAME srvpro_api::plugin::shout
import { useState } from 'preact/hooks'
import type { ComponentType } from 'preact'
import { NotificationOutlined } from '@ant-design/icons'
import { App as AntdApp, Button, Input, Modal, Tooltip } from 'antd'
import type {} from '@srvpro/portal-sdk'

const TITLE = '广播'

export function Shout() {
  const [open, set_open] = useState(false)
  const [draft, set_draft] = useState('')
  const [pending, set_pending] = useState(false)
  const { message } = AntdApp.useApp()

  const close = () => { set_open(false); set_draft('') }

  const submit = async () => {
    set_pending(true)
    try {
      const response = await srvpro_fetch(`/api/shout?shout=${encodeURIComponent(draft)}`)
      if (!response.ok) throw new Error(`广播失败：${response.status}`)
      message.success(`${TITLE}成功`)
      close()
    } catch (failure) {
      message.error(failure instanceof Error ? failure.message : `${TITLE}失败`)
    } finally {
      set_pending(false)
    }
  }

  return (
    <>
      <Tooltip title={TITLE}><Button type="text" icon={<NotificationOutlined />} onClick={() => set_open(true)} /></Tooltip>
      {open && (
        <Modal
          open
          title={TITLE}
          okText="确定"
          okButtonProps={{ disabled: draft.trim() === '' }}
          confirmLoading={pending}
          closeIcon={null}
          footer={(_: unknown, { OkBtn }: { OkBtn: ComponentType }) => <OkBtn />}
          onOk={submit}
          onCancel={close}
        >
          <Input autoFocus value={draft} placeholder="广播给所有房间的内容" onChange={(event: Event) => set_draft((event.target as HTMLInputElement).value)} onPressEnter={submit} />
        </Modal>
      )}
    </>
  )
}
