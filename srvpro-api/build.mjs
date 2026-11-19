import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { build } from 'vite'
import { define_page_config } from '@srvpro/portal-sdk/vite'

const PATTERN = /^\/\/\s*SRVPRO_PLUGIN_NAME\s+(\S+)/
const OUT_DIR = join(import.meta.dirname, 'dist')

for (const base of ['src', 'src/plugin']) {
  for (const entry of await readdir(join(import.meta.dirname, base), { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.tsx')) continue
    const file = join(import.meta.dirname, base, entry.name)
    const match = PATTERN.exec(await readFile(file, 'utf8'))
    if (match === null) continue
    await build(define_page_config({ entry: file, fileName: match[1], outDir: OUT_DIR }))
  }
}

