import { useEffect, useState } from 'preact/hooks'
import Router, { Route } from 'preact-router'
import type { ComponentChildren } from 'preact'
import { App as AntdApp, Card, ConfigProvider, Space } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import type { PortalConfigTab, PortalIcon, PortalManifest } from '@srvpro/portal-sdk'
import { Settings } from './settings'
import { Slot } from './slot'

export function App({ manifest }: { manifest: PortalManifest | null }) {
  const [server_name, set_server_name] = useState('Srvpru')

  useEffect(() => {
    srvpro_fetch('/api/config?prefix=api_name')
      .then((response) => response.json() as Promise<Record<string, string>>)
      .then((values) => { if (values.api_name) set_server_name(values.api_name) })
      .catch(() => undefined)
  }, [])
  useEffect(() => { document.title = server_name }, [server_name])

  return (
    <ConfigProvider locale={zhCN}>
      <AntdApp component={false}>
        <Router>
          {(manifest?.pages ?? []).filter((page) => page.route !== '').map((page) => (
            <Route
              key={page.route}
              path={page.route}
              component={() => page.topbar
                ? <TopBar title={server_name} icons={manifest?.icons ?? []} tabs={manifest?.tabs ?? []}><Slot plugin={page.plugin} name={page.export} /></TopBar>
                : <Slot plugin={page.plugin} name={page.export} />}
            />
          ))}
        </Router>
      </AntdApp>
    </ConfigProvider>
  )
}
 
function TopBar({ title, icons, tabs, children }: { title: string; icons: PortalIcon[]; tabs: PortalConfigTab[]; children: ComponentChildren }) {
  return (
    <div class="room">
      <Card title={title} extra={<Space size={4}>{icons.map((icon) => <Slot key={icon.plugin} plugin={icon.plugin} name={icon.export} />)}<Settings tabs={tabs} /></Space>}>
        {children}
      </Card>
    </div>
  )
}
