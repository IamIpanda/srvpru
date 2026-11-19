import { readFileSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { defineConfig, type Plugin } from 'vite'
import { IMPORT_MAP, is_host_module } from '@srvpro/portal-sdk/shared'

const DIST_DIRECTORY = fileURLToPath(new URL('../srvpro-api/dist/', import.meta.url))
const MODULE_NAME = /^\/([^/]+\.js)$/

/**
 * The import map has to be in the document before any module script runs, and
 * it only makes sense for the built bundle: the dev server resolves preact and
 * antd itself, through a single pre-bundled copy.
 */
function import_map_plugin(): Plugin {
  return {
    name: 'srvpro-import-map',
    apply: 'build',
    transformIndexHtml() {
      return [{ tag: 'script', attrs: { type: 'importmap' }, children: JSON.stringify({ imports: IMPORT_MAP }), injectTo: 'head-prepend' }]
    },
  }
}

/**
 * Plugin bundles keep their bare imports of the host modules, so in dev they have to
 * go through Vite as well; otherwise the browser resolves `preact/hooks` nowhere, or
 * resolves it to a second copy of preact.
 *
 * They are build output that Vite never watches, so their module graph entries have to
 * be dropped by hand whenever the file on disk moved on.
 */
function page_modules_plugin(): Plugin {
  const modified = new Map<string, number>()

  return {
    name: 'srvpro-page-modules',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((request, _response, next) => {
        const name = MODULE_NAME.exec(request.url?.split('?')[0] ?? '')?.[1]
        if (name !== undefined) {
          const path = `${DIST_DIRECTORY}${name}`
          const mtime = statSync(path, { throwIfNoEntry: false })?.mtimeMs
          if (mtime !== undefined && modified.get(path) !== mtime) {
            modified.set(path, mtime)
            for (const module of server.moduleGraph.getModulesByFile(path) ?? []) server.moduleGraph.invalidateModule(module)
          }
        }
        next()
      })
    },
    resolveId(specifier: string) {
      if (!specifier.startsWith('/')) return null
      const path = `${DIST_DIRECTORY}${specifier.slice(1)}`
      return statSync(path, { throwIfNoEntry: false }) === undefined ? null : path
    },
    load(id: string) {
      const [path] = id.split('?')
      if (!path.startsWith(DIST_DIRECTORY)) return null
      return readFileSync(path, 'utf-8')
    },
  }
}

export default defineConfig({
  plugins: [import_map_plugin(), page_modules_plugin()],
  resolve: {
    alias: {
      'react/jsx-runtime': 'preact/jsx-runtime',
      'react/jsx-dev-runtime': 'preact/jsx-runtime',
      'react-dom/test-utils': 'preact/test-utils',
      react: 'preact/compat',
      'react-dom': 'preact/compat',
    },
  },
  oxc: { jsx: { runtime: 'automatic', importSource: 'preact' } },
  build: {
    target: 'es2022',
    rollupOptions: { external: (specifier: string) => is_host_module(specifier) },
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:7922',
      '/roomlist': { target: 'http://127.0.0.1:7922', ws: true },
    },
  },
})
