// Application response semantics, shared by browser and fragment captures.
import { createHash } from 'node:crypto'
import { maskText, normalizeResponse } from './normalize.ts'
import type { NormalizeOptions } from './normalize.ts'

// Hop framing, implementation identity and instrumentation differ across Rails/Rust and engines.
const TRANSPORT = new Set(['connection', 'content-length', 'keep-alive', 'server', 'server-timing', 'transfer-encoding', 'x-runtime'])
const SESSION_COOKIES = new Set(['_campfire_session', 'session_token'])
export type Header = { name: string; value: string }

export function responseHeaders(headers: Header[], options: NormalizeOptions = {}, body?: Buffer): string {
  const html = headers.some(h => h.name.toLowerCase() === 'content-type' && /(?:text\/html|turbo-stream)/i.test(h.value))
  // WebKit splits comma-separated fields in headersArray(), including HTTP dates. Restore
  // their field value before normalization; Set-Cookie is never comma-joined (Expires has one).
  const fields = new Map<string, string[]>()
  const cookies: Header[] = []
  for (const header of headers) {
    const name = header.name.toLowerCase()
    if (name === 'set-cookie') cookies.push(header)
    else fields.set(name, [...(fields.get(name) ?? []), header.value])
  }
  const combined = [...fields].map(([name, values]) => ({ name, value: values.join(', ') }))
  return [...combined, ...cookies].flatMap(({ name, value }) => {
    name = name.toLowerCase()
    if (TRANSPORT.has(name)) return []
    if (name === 'set-cookie') return [`set-cookie: ${cookie(value, options)}`]
    if (name === 'date') value = value.replace(/(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} [A-Z][a-z]{2} \d{4} \d{2}:\d{2}:\d{2} GMT/g, date => Number.isNaN(Date.parse(date)) ? date : '«response-date»')
    else if (name === 'x-request-id') value = value ? '«request-id»' : value
    else if (name === 'x-csrf-token') value = value ? '«csrf-token»' : value
    else if (name === 'content-security-policy' || name === 'content-security-policy-report-only') value = value.replace(/'nonce-[^']+'/g, "'nonce-«nonce»'")
    else if (name === 'x-cache' && /^(hit|miss)$/i.test(value)) value = '«cache-hit-or-miss»'
    else if (name === 'etag' && html && body && value === `W/"${createHash('sha256').update(body).digest('hex').slice(0, 32)}"`) {
      // Rack's weak HTML validator hashes fresh CSRF/nonces too. Verify that it really names
      // these bytes before substituting their normalized digest. An invented/wrong ETag fails.
      value = `W/"«normalized-sha256:${createHash('sha256').update(normalizeResponse(body, 'text/html', options)).digest('hex')}»"`
    }
    else if (name === 'cache-control') {
      // Directive names are case-insensitive; quoted extension values are not. Keep commas
      // inside quoted strings and preserve value bytes while canonicalizing name/order.
      value = (value.match(/(?:[^,"]|"(?:\\.|[^"\\])*")+/g) ?? [value]).map(directive => {
        const equal = directive.indexOf('=')
        return equal < 0 ? directive.trim().toLowerCase() : `${directive.slice(0, equal).trim().toLowerCase()}=${directive.slice(equal + 1).trim()}`
      }).sort().join(', ')
    }
    else if (name === 'vary') value = value.split(',').map(s => s.trim().toLowerCase()).sort().join(', ')
    else if (name === 'location') value = maskText(value, options)
    return [`${name}: ${value}`]
  }).sort().join('\n')
}

function cookie(raw: string, options: NormalizeOptions): string {
  const [pair, ...attrs] = raw.split(';').map(s => s.trim())
  const equal = pair.indexOf('=')
  const name = equal < 0 ? pair : pair.slice(0, equal)
  let value = equal < 0 ? '' : pair.slice(equal + 1)
  // Only the two actual session transports contain fresh opaque signed/encrypted bytes.
  // Empty/deletion values, other cookie values, and every attribute remain significant.
  if (value && SESSION_COOKIES.has(name)) value = '«session-value»'
  const attributes = attrs.map(attr => {
    const i = attr.indexOf('=')
    const key = (i < 0 ? attr : attr.slice(0, i)).toLowerCase()
    let value = i < 0 ? undefined : attr.slice(i + 1)
    if (key === 'expires' && value && options.seedTime !== undefined && !Number.isNaN(Date.parse(value))) {
      value = `«seed${(Date.parse(value) - options.seedTime) / 1000 >= 0 ? '+' : ''}${(Date.parse(value) - options.seedTime) / 1000}s»`
    }
    if (key === 'samesite' && value) value = value.toLowerCase()
    return key + (value === undefined ? '' : `=${value}`)
  }).sort()
  return [`${name}=${value}`, ...attributes].join('; ')
}
