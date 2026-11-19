import type { PortalManifest } from '@srvpro/portal-sdk'

export async function fetch_manifest(): Promise<PortalManifest> {
  const response = await fetch('/api/manifest')
  if (!response.ok) throw new Error(`无法读取门户清单：${response.status}`)
  return (await response.json()) as PortalManifest
}
