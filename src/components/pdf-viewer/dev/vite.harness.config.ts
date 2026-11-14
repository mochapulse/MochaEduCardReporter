/**
 * Build config for the dev-only viewer harness (not used by the app build):
 *   pnpm exec vite build --config src/components/pdf-viewer/dev/vite.harness.config.ts
 * Output goes to target/pdf-viewer-harness/, to be served with the production CSP.
 */
import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const root = path.resolve(import.meta.dirname, '../../../..')

export default defineConfig({
  root,
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(root, 'src') } },
  build: {
    outDir: path.resolve(root, 'target/pdf-viewer-harness'),
    emptyOutDir: true,
    // The sample PDFs are fetched; a data: URL would be refused by connect-src.
    assetsInlineLimit: 0,
    rollupOptions: { input: path.resolve(import.meta.dirname, 'harness.html') },
  },
})
