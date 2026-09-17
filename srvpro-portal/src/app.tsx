import Router from 'preact-router'
import { Route } from 'preact-router'
import { ConfigProvider } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import { Home } from './pages/home'
import { Login } from './pages/login'

export function App() {
  return (
    <ConfigProvider locale={zhCN}>
      <Router>
        <Route path="/login" component={Login} />
        <Route default component={Home} />
      </Router>
    </ConfigProvider>
  )
}
