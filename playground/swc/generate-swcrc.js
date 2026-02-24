const path = require('path')
const { writeFileSync } = require('fs')
const wasmPath = path.resolve(__dirname, '..', '..', 'target', 'wasm32-wasip1', 'debug', 'swc_plugin_coverage.wasm')

const config = {
  $schema: 'https://json.schemastore.org/swcrc',
  jsc: {
    experimental: {
      plugins: [[wasmPath, {}]],
    },
  },
}

const configStr = JSON.stringify(config, null, 2)
writeFileSync(path.join(__dirname, '.swcrc'), configStr)
