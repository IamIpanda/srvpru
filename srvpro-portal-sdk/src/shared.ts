/**
 * The modules the host owns. Pages must never bundle them: a second copy of
 * preact breaks hooks and a second copy of antd breaks its context, both
 * silently, as a blank page.
 *
 * Every entry name here is also the file name the vendor build emits, so
 * `IMPORT_MAP` and `VENDOR_ENTRIES` have to stay in step.
 */
export const VENDOR_ENTRIES: Record<string, string> = {
  preact: 'preact',
  hooks: 'preact/hooks',
  'jsx-runtime': 'preact/jsx-runtime',
  compat: 'preact/compat',
  'preact-router': 'preact-router',
  antd: 'antd',
  'antd-locale-zh-cn': 'antd/locale/zh_CN',
  icons: '@ant-design/icons',
}

export const IMPORT_MAP: Record<string, string> = {
  preact: '/vendor/preact.js',
  'preact/hooks': '/vendor/hooks.js',
  'preact/jsx-runtime': '/vendor/jsx-runtime.js',
  'preact/compat': '/vendor/compat.js',
  'preact-router': '/vendor/preact-router.js',
  antd: '/vendor/antd.js',
  'antd/locale/zh_CN': '/vendor/antd-locale-zh-cn.js',
  '@ant-design/icons': '/vendor/icons.js',
  react: '/vendor/compat.js',
  'react-dom': '/vendor/compat.js',
}

export const HOST_MODULES: readonly string[] = Object.keys(IMPORT_MAP)

export const FORBIDDEN_PREFIXES: readonly string[] = ['preact/', 'antd/', 'react/', 'react-dom/', '@ant-design/icons/']

export function is_host_module(specifier: string): boolean { return HOST_MODULES.includes(specifier) }
