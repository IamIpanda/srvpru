import { render } from 'preact'
import 'antd/dist/reset.css'
import type { PortalManifest } from '@srvpro/portal-sdk'
import { App } from './app'
import './index.css'
import { fetch_manifest } from './portal'

async function bootstrap(): Promise<PortalManifest | null> {
  const manifest = await fetch_manifest().catch(() => null)
  await Promise.all((manifest?.bootstraps ?? []).map((entry) => import(/* @vite-ignore */ `/${entry.plugin}.js`).catch(() => undefined)))
  return manifest
}

const manifest = await bootstrap()
render(<App manifest={manifest} />, document.getElementById('app')!)
