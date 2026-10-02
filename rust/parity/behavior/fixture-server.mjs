// Test-only fixture bridge. Rails' own models prepare both isolated databases;
// the browser still exercises the Rust/Rails HTTP dispatcher, never a fake app route.
import assert from "node:assert/strict"
import { realpath } from "node:fs/promises"
import { createServer } from "node:net"
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { resolve, sep } from "node:path"

const [socket, requestedStorage, instant] = process.argv.slice(2)
const root = await realpath(resolve(import.meta.dirname, "../../.."))
const storage = await realpath(requestedStorage)
assert(storage.startsWith(`${root}/.scratch/`) || storage.startsWith(`${root}/rust/parity/.seed/.instances/`),
  "only an isolated app storage copy in this worktree is allowed")
assert(!storage.includes(`${sep}..${sep}`))
const execute = promisify(execFile)
const server = createServer(connection => {
  let request = ""
  connection.setEncoding("utf8")
  connection.on("data", async chunk => {
    request += chunk
    if (!request.includes("\n")) return
    connection.pause()
    try {
      assert.equal(JSON.parse(request).operation, "register_late")
      const { stdout } = await execute("bash", ["rust/parity/bin/reference", "runner", "--storage", storage,
        "--time", instant, "--freeze", "rust/parity/behavior/register-live.rb", "/work/parity/.seed/default/labels.json"],
      { cwd: root, timeout: 30_000, maxBuffer: 1_048_576 })
      console.log(stdout.trim())
      connection.end(`${JSON.stringify({ ok: true })}\n`)
    } catch (error) { connection.end(`${JSON.stringify({ ok: false, error: String(error) })}\n`) }
  })
  connection.on("error", () => {})
})
server.listen(socket)
