#!/usr/bin/env node

const fs = require('fs')
const { run } = require('./index')

const input = process.argv[2]
const output = process.argv[3]

if (!input || !output) {
  console.error(`usage:
prepare-swagger path_to_config.yaml path_to_schema.yaml`)
  process.exit(1)
}

try {
  const config = fs.readFileSync(input, 'utf8')
  const schema = run(config)
  fs.writeFileSync(output, schema, 'utf8')
  process.exit(0)
} catch (error) {
  console.error(error)
  process.exit(1)
}
