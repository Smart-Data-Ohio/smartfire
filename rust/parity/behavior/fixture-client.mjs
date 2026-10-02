import assert from "node:assert/strict"
import { connect } from "node:net"

export async function registerLiveCommand() {
  await new Promise((resolve, reject) => {
    const connection = connect("/bridge/fixtures.sock")
    let response = ""
    // Guards fixture-process startup, not any browser/application assertion.
    connection.setTimeout(30_000, () => connection.destroy(new Error("Rails fixture process did not complete")))
    connection.on("connect", () => connection.write(`${JSON.stringify({ operation: "register_late" })}\n`))
    connection.setEncoding("utf8")
    connection.on("data", chunk => { response += chunk })
    connection.on("error", reject)
    connection.on("end", () => {
      try { const result = JSON.parse(response); assert(result.ok, result.error); resolve() } catch (error) { reject(error) }
    })
  })
}
