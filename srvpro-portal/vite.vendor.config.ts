import { defineConfig } from 'vite'
import { VENDOR_ENTRIES } from '@srvpro/portal-sdk/shared'

export default defineConfig({
  define: { 'process.env.NODE_ENV': '"production"' },
  resolve: {
    alias: {
      'react/jsx-runtime': 'preact/jsx-runtime',
      'react/jsx-dev-runtime': 'preact/jsx-runtime',
      'react-dom/test-utils': 'preact/test-utils',
      react: 'preact/compat',
      'react-dom': 'preact/compat',
    },
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: false,
    lib: { entry: VENDOR_ENTRIES, formats: ['es'] },
    rollupOptions: {
      input: VENDOR_ENTRIES,
      output: {
        entryFileNames: 'vendor/[name].js',
        chunkFileNames: 'vendor/[name]-[hash].js',
        assetFileNames: 'vendor/[name]-[hash][extname]',
      },
    },
  },
})
