declare global {
  var srvpro_fetch: typeof fetch
}

export type PortalPage = {
  plugin: string
  route: string
  export: string
  topbar: boolean
}

export type PortalBootstrap = {
  plugin: string
}

export type PortalIcon = {
  plugin: string
  order: number
  export: string
}

export type PortalConfigFieldKind = 'text' | 'multiline' | 'boolean' | 'select' | 'tags' | 'json'

export type PortalConfigField = {
  key: string
  label: string
  kind: PortalConfigFieldKind
  default: string
  options: [string, string][]
}

export type PortalConfigTabContent = { fields: PortalConfigField[] } | { component: { export: string } }

export type PortalConfigTab = {
  plugin: string
  title: string
  order: number
  toggle: boolean
  content: PortalConfigTabContent
}

export type PortalManifest = {
  bootstraps: PortalBootstrap[]
  pages: PortalPage[]
  icons: PortalIcon[]
  tabs: PortalConfigTab[]
}
