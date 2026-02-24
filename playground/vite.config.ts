import path from 'path'
import { fileURLToPath } from 'url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// https://vite.dev/config/
export default defineConfig({
  plugins: [react({
    plugins: [
      [path.resolve(__dirname, '..', 'target', 'wasm32-wasip1', 'release', 'swc_plugin_coverage.wasm'), {}]
    ]
  })],
})
