import { defineConfig, type Plugin, type UserConfig } from 'vite'
import { FORBIDDEN_PREFIXES } from './shared.ts'
import { is_host_module } from './shared.ts'

/**
 * Deep imports such as `antd/es/button` cannot be mapped by the import map,
 * so they have to fail the build instead of producing a page that white
 * screens at runtime.
 */
function host_modules_plugin(): Plugin {
  return {
    name: 'srvpro-host-modules',
    apply: 'build',
    enforce: 'pre',
    resolveId(specifier: string) {
      if (is_host_module(specifier)) return { id: specifier, external: true }
      if (!FORBIDDEN_PREFIXES.some((prefix) => specifier.startsWith(prefix))) return null
      throw new Error(`${specifier} is not a host module: the import map only covers the top level of preact, antd, react and react-dom`)
    },
  }
}

export type PageConfig = {
  entry: string
  fileName: string
  outDir: string
}

export function define_page_config(configuration: PageConfig): UserConfig {
  return defineConfig({
    oxc: { jsx: { runtime: 'automatic', importSource: 'preact' } },
    plugins: [host_modules_plugin()],
    build: {
      target: 'es2022',
      outDir: configuration.outDir,
      emptyOutDir: false,
      cssCodeSplit: false,
      lib: { entry: configuration.entry, formats: ['es'], fileName: configuration.fileName, cssFileName: configuration.fileName },
    },
  })
}
