// Fault injection at the saved screenshot boundary; all server-output layers stay identical.
import fs from 'node:fs'
import { syncBuiltinESMExports } from 'node:module'
import { PNG } from 'pngjs'
const write = fs.writeFileSync
let injected = false
fs.writeFileSync = ((file: any, data: any, ...args: any[]) => {
  if (!injected && String(file).includes('/actual/') && String(file).endsWith('.png')) {
    const png = PNG.sync.read(data)
    png.data[0] ^= 255; png.data[1] ^= 255; png.data[2] ^= 255
    data = PNG.sync.write(png)
    injected = true
  }
  return (write as any)(file, data, ...args)
}) as typeof fs.writeFileSync
syncBuiltinESMExports()
