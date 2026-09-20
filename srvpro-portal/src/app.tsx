import Router from 'preact-router'
import { Route } from 'preact-router'
import { App as AntdApp } from 'antd'
import { ConfigProvider } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import { Home } from './pages/home'
import { Login } from './pages/login'
import { Room } from './pages/room'

export function App() {
  return (
    <ConfigProvider locale={zhCN}>
      <AntdApp component={false}>
        <Router>
          <Route path="/login" component={Login} />
          <Route path="/" component={Room} />
          <Route default component={Home} />
        </Router>
      </AntdApp>
    </ConfigProvider>
  )
}
