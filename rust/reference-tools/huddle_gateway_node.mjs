// Test-only adapter for OUR gateway.test.mjs. Gateway code and assertions are unchanged.
// Ordinary admissions/checks/leaves reach Rust; explicit outage, stall and malformed-body
// injections remain at the proxy. Denial injections revoke the real persisted grant.
import { registerHooks } from "node:module";
import { pathToFileURL } from "node:url";

function replaceOne(source, before, after) {
  if (!source.includes(before) || source.indexOf(before) !== source.lastIndexOf(before)) throw new Error(`gateway adapter anchor drifted: ${before}`);
  return source.replace(before, after);
}
registerHooks({
  resolve(specifier, context, next) {
    if (specifier === "ws") return { url: pathToFileURL(process.env.WS13_WS_MODULE).href, shortCircuit: true };
    return next(specifier, context);
  },
  load(url, context, next) {
    const loaded = next(url, context);
    if (!url.endsWith("/script/livekit-gateway/gateway.test.mjs")) return loaded;
    let source = loaded.source.toString();
    source = replaceOne(source, 'const TOKEN = "header.payload.signature";', 'const TOKEN = process.env.WS13_JOIN_TOKEN;');
    source = replaceOne(source, 'const GRANT = Object.freeze({ grant_id: 17, room_name: "opaque-room", identity: "opaque-identity" });', 'const GRANT = Object.freeze({ grant_id: 17, room_name: "ws13-security-room", identity: "ws13-security-participant" });');
    source = replaceOne(source, 'const SECRET = "gateway-secret-for-tests";', 'const SECRET = "ws13-fixture-gateway-secret";');
    source = replaceOne(source, 'const API_KEY = "livekit-api-key-for-tests";', 'const API_KEY = "ws13-fixture-api-key";');
    source = replaceOne(source, 'const API_SECRET = "livekit-api-secret-for-tests-which-is-long";', 'const API_SECRET = "ws13-fixture-api-secret";');
    source = replaceOne(source, '  const state = {', `  const fixtureResponse = await fetch(process.env.WS13_FIXTURE_CONTROL + "/start", { method: "POST" });
  assert.equal(fixtureResponse.status, 200);
  const fixture = await fixtureResponse.json();
  const rustRequests = new Set();
  const rustFetch = (request, response, body) => {
    const pending = (async () => {
    const upstream = await fetch(fixture.url + request.url, { method: request.method,
      headers: { "content-type": "application/json", "x-huddle-gateway-secret": request.headers["x-huddle-gateway-secret"] ?? "",
        ...(request.headers.authorization ? { authorization: request.headers.authorization } : {}) },
      ...(request.method === "POST" ? { body: JSON.stringify(body ?? {}) } : {}) });
    assert.equal(upstream.headers.get("cache-control"), "no-store");
    response.writeHead(upstream.status, {"content-type": upstream.headers.get("content-type") ?? "application/json"});
    response.end(await upstream.text());
    })();
    rustRequests.add(pending);
    pending.finally(() => rustRequests.delete(pending));
    return pending;
  };
  const revoke = async () => {
    assert.equal((await fetch(fixture.url + "/__ws13/revoke", {method: "POST"})).status, 200);
  };
  const state = {`);
    source = replaceOne(source, '      await readJson(request);', '      await readJson(request);');
    source = replaceOne(source, '        if (result) return json(response, result.status, result.body ?? {});', `        if (result) {
          if (result.status === 403) { await revoke(); return rustFetch(request, response); }
          return json(response, result.status, result.body ?? {});
        }`);
    source = replaceOne(source, '      return json(response, state.authorizeStatus, state.authorizeStatus === 200 ? GRANT : {});', `      if (state.authorizeStatus === 403) await revoke();
      else if (state.authorizeStatus !== 200) return json(response, state.authorizeStatus, {});
      return rustFetch(request, response);`);
    source = replaceOne(source, '      return json(response, status, status === 200 ? GRANT : {});', `      if (status === 403) await revoke();
      else if (status !== 200) return json(response, status, {});
      return rustFetch(request, response);`);
    source = replaceOne(source, '      return json(response, state.leftStatus, {});', `      if (state.leftStatus !== 200) return json(response, state.leftStatus, {});
      return rustFetch(request, response, body);`);
    // Keep the original test servers inside WS13's assigned range too.
    source = replaceOne(source, '  server.listen(0, "127.0.0.1");\n  await once(server, "listening");\n  return server.address().port;', `  for (let port = 52340; port <= 52399; port++) {
    const result = await new Promise(resolve => {
      const error = () => { server.off("listening", ready); resolve(false); };
      const ready = () => { server.off("error", error); resolve(true); };
      server.once("error", error); server.once("listening", ready); server.listen(port, "127.0.0.1");
    });
    if (result) return server.address().port;
  }
  throw new Error("WS13 gateway fixture port range exhausted");`);
    source = replaceOne(source, '    listenPort: 0,', '    listenPort: await availablePort(),');
    source += `\nasync function availablePort() {
      const probe = http.createServer();
      const port = await listen(probe); await closeServer(probe); return port;
    }\n`;
    source = replaceOne(source, '      await closeServer(livekit);', `      await closeServer(livekit);
      await Promise.all([...rustRequests]);
      const persisted = await (await fetch(fixture.url + "/__ws13/state")).json();
      if (state.leftPosts.length && state.leftStatus === 200 && state.upstreamConnections === 1) assert.equal(persisted.seen, null);
      if (!state.active || state.authorizeStatus === 403 || state.grantCheckStatus === 403) assert.equal(persisted.revoked, true);
      assert.equal((await fetch(fixture.url + "/__ws13/close", {method: "POST"})).status, 200);`);
    return { ...loaded, source };
  },
});
