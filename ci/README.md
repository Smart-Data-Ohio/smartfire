# Continuous integration

Every test kept in the repository runs on pull requests that change its inputs.
There is no nightly or heavy test tier. Browser and messaging correctness suites,
their isolated hosts, and their unused recorded fixtures have been removed.
Ordinary tests retain the frozen seeds, golden vectors and fixtures they read.

`Rust port` reports on every pull request and main push. `Rust changes` selects
Rust source, Cargo, crate, CI, static asset and fixture inputs. An unrelated PR
skips the build jobs and passes the aggregate without collecting test artifacts.
Unavailable history runs the checks. Main pushes and manual Rust dispatches run
all Rust checks.

`Rust source` resolves the tested revision once. Checkouts and artifacts use that
full SHA. Reruns restore the first attempt's pin rather than following a moved
branch. Missing or mismatched pins fail and require a new run.

| Job | Work |
| --- | --- |
| Rust seeds | Verify the committed seeds against their manifest and schema. |
| Rust tests (1/2), Rust tests (2/2) | Run every ordinary workspace test in two nextest partitions. Both use `ci/llvm.toml` so panic-recovery tests can unwind. |
| Rust clippy, binaries and doctests | Run CI helper tests, clippy, generated TypeScript checks, the production-input binary build and runnable doctests. |
| Rust port | Require all selected jobs to succeed and reconcile the shards' JUnit receipts with the complete nextest list. |

`check_gate_needs.py` rejects any Rust job outside the aggregate's dependencies.
`summarize-tests.py` requires each selected test to pass exactly once and checks
the doctest logs. The setup action restores committed seeds before app tests.
`ci/cargo.sh` uses the Dockerfile's toolchain and media libraries, four build
slots, and the CI profile in `cargo-config.toml`. Local builds keep their normal
profile and linker. Cargo caches retain dependencies; workspace outputs are
pruned before a main push saves a new cache.

Frontend auth inputs under `frontend/src/auth`, `styles`, `motion`, and the
shared button, text-field and checkbox styles also feed Rust's asset build.
Generated API types and embedded dist files are Rust inputs too.

The other required checks are `Frontend`, `GitHub Actions audit`,
`Huddle authorization gateway`, `Ops scripts` and `Dependency audit`. Their
jobs report on every PR and main push and skip expensive steps internally when
irrelevant. The Frontend job runs lint, typecheck, Vitest and a production build,
then tests `crates/spa` with that dist. Its three advisory e2e shards use the
Vite mock server, which does not compile Rust. One shard also runs the production
preview tests.

Only Dependency audit has a weekly schedule in the repository-check workflow.
CodeQL runs weekly or manually. Operational backup and restore schedules remain
independent of PR tests.

Deploy checks a successful Rust main-push run before publishing a missing
immutable `rust-git-<sha>` amd64 image to Artifact Registry. It resolves that tag
to a digest for every release phase. Healthy cutovers verify the public auth
pages and built SPA assets with HTTP; a disagreement returns exit 20 and does
not restore the database.
