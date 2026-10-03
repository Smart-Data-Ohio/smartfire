# Complete proxy header oracle

Pinned Rails is captured through its actual `config.ru` Rack::Deflater entrypoint.
Every request asserts SERVER_PROTOCOL=HTTP/1.1 and preserves that protocol in its
receipt. Rack responses retain every header name and an array of every value;
case-insensitive duplicate names and non-singleton values fail during capture.

There are21 representation responses (success, missing preview, missing variant,
redirect, disk and proxy) and4 blob-proxy controls (200,206,416,404). Rust executes
its real router. Comparisons retain all status/body bytes and every stable header,
including CSP, Content-Transfer-Encoding, Content-Range and Vary. Only the six
explicitly approved security additions and Date/X-Request-Id/X-Runtime values
have special handling. Single-value cardinality is checked before those names
are handled; dates, UUIDv4 IDs and six-place nonnegative runtimes are validated.
Fixed CSP nonce-generator entropy, storage keys and mtimes are fixture inputs;
production nonce generation is unaffected. There is no CSP response mask.

The new Rust all-header checker rejected the old nine-name projection before
recapturing the oracle:

```text
thread 'controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_proxy' panicked: header values
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2569 filtered out; finished in 0.87s
```

The complete blob differential passes:

```text
WS11 blob proxy: 4 responses; all headers/body bytes; rows/jobs unchanged
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2579 filtered out; finished in 1.97s
```

`test-proxy-headers.py` rejects every changed ordinary field in its fixture,
unexpected/missing names, duplicate names or values (including approved names),
changed security values and applying the proxy approval to other routes. The
Rust `ws11_proxy_header_oracle_rejects_unapproved_changes` additionally challenges
the same comparator used by the actual HTTP regressions for all four blob controls.
`test-http-vector-verifier.py` challenges complete captured representations too.
The oracle has no fixed response-header projection.

Reproduction commands are included in ws11api-next-2-report.md. The native video
content-length difference remains visible (3336 versus3326); pinned runtime is
required to match encoded media bytes. No size, checksum or length field is masked.
