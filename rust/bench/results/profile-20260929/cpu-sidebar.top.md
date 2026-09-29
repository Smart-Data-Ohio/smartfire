# campfire main: sidebar

23851 samples at 1000 µs (23.85 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **gzip (miniz_oxide/crc32)** | (all) | **41.2%** |
| **sqlite (C)** | (all) | **14.8%** |
| **app (controllers/channels/jobs)** | (all) | **10.9%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 3.5% |
| **tokio runtime / scheduling** | (all) | **7.8%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 3.0% |
| **askama render / view helpers** | (all) | **7.1%** |
| askama render / view helpers | malloc/free/realloc | 0.8% |
| **kit (request/response plumbing)** | (all) | **6.2%** |
| **crypto / signing (rails_compat)** | (all) | **5.5%** |
| **db models / queries** | (all) | **4.2%** |
| db models / queries | syscalls (read/write/epoll/futex) | 3.0% |
| **hyper / http** | (all) | **1.7%** |
| **json (serde_json)** | (all) | **0.5%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 9.7% | `zlib_rs::longest_match_help<false>` |
| 6.2% | `syscall` |
| 6.1% | `zlib_rs::is_match<8>` |
| 4.7% | `libc.so.6+0x7795b1aa0952` |
| 2.6% | `zlib_rs::insert_string` |
| 2.1% | `zlib_rs::deflate::algorithm::medium::deflate_medium` |
| 1.6% | `zlib_rs::send_bits` |
| 1.6% | `libsqlite3_sys::sqlite3VdbeExec` |
| 1.5% | `<zlib_rs::deflate::Heap>::pqdownheap` |
| 1.4% | `zlib_rs::braid_core<5>` |
| 1.4% | `zlib_rs::quick_insert_value` |
| 1.3% | `core::next<u8>` |
| 1.2% | `core_arch::_mm_add_epi32` |
| 1.0% | `imalloc_fastpath` |
| 1.0% | `core_arch::_mm_shuffle_epi32<14>` |
| 0.9% | `zlib_rs::hash_calc` |
| 0.9% | `zlib_rs::deflate::longest_match::longest_match` |
| 0.8% | `core::split_at_checked<u8>` |
| 0.8% | `_rjem_je_arena_ralloc_no_move` |
| 0.8% | `core_arch::_mm_alignr_epi8<4>` |
| 0.8% | `cache_bin_alloc_impl` |
| 0.7% | `libsqlite3_sys::columnName` |
| 0.7% | `core::index<u8>` |
| 0.7% | `core::eq` |
| 0.7% | `libc.so.6+0x7795b1a94f67` |
| 0.6% | `zlib_rs::insert_match` |
| 0.6% | `libc.so.6+0x7795b1b8fd07` |
| 0.5% | `core_arch::_mm_sha256msg1_epu32` |
| 0.5% | `libc.so.6+0x7795b1a9d3bc` |
| 0.5% | `zlib_rs::as_slice<u16, 65536>` |
| 0.5% | `sz_index2size_lookup_impl` |
| 0.5% | `campfire_views::helpers::html::push_escaped` |
| 0.5% | `_rjem_je_arena_ralloc` |
| 0.5% | `free_fastpath` |
| 0.4% | `libsqlite3_sys::sqlite3_column_count` |
| 0.4% | `copied<[u8, 8], core::array::TryFromSliceError>` |
| 0.4% | `libsqlite3_sys::columnMem` |
| 0.4% | `<core::fmt::Arguments>::estimated_capacity` |
| 0.4% | `libc.so.6+0x7795b1b8fd0e` |
| 0.4% | `_rjem_realloc` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `libc.so.6+0x7795b1a980a1` |
| 100.0% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 100.0% | `tokio::{closure}` |
| 100.0% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 100.0% | `libc.so.6+0x7795b1b2080b` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 100.0% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 100.0% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 100.0% | `tokio::run` |
| 100.0% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 100.0% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 100.0% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 96.7% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 96.7% | `tokio::poll` |
| 78.1% | `poll<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blocking:` |
| 78.1% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 78.1% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 78.1% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 78.1% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 78.1% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 78.1% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 78.1% | `set<tokio::runtime::scheduler::Context, tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}, ()>` |
| 78.1% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 78.1% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 78.1% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 78.1% | `enter_runtime<tokio::runtime::scheduler::multi_thread::worker::run::{closure_env#0}, ()>` |
| 78.1% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 78.1% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 78.1% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 78.1% | `tokio::runtime::task::raw::poll::<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 78.1% | `poll<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}, ()>` |
| 77.8% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 77.8% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 77.8% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 77.7% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 77.7% | `tokio::runtime::task::raw::poll::<campfire_kit::front::conn::accept_loop<campfire_kit::front::serve_plain::{closure}::{closure}, campfire_kit::front::conn::serv` |
| 77.7% | `poll<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_block_env#0}<campfire_kit::front::serve_plain::{async_fn#0}::{closure_env#0}, campfire_kit::fr` |
| 77.7% | `poll_inner<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_block_env#0}<campfire_kit::front::serve_plain::{async_fn#0}::{closure_env#0}, campfire_k` |
| 77.7% | `poll_future<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_block_env#0}<campfire_kit::front::serve_plain::{async_fn#0}::{closure_env#0}, campfire_` |
| 77.7% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_block_env#0}<campfire_kit::front::serve_plain::{async_fn` |
| 77.7% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::fr` |
| 77.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::front::conn::accept_loop::{async_fn` |
| 77.7% | `{closure}<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_block_env#0}<campfire_kit::front::serve_plain::{async_fn#0}::{closure_env#0}, campfire_ki` |
| 77.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::front::conn::accept_loop::{asy` |
| 77.7% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::front::conn::accept_loop::{async_fn#0}::{async_b` |
| 77.7% | `campfire_kit::front::conn::accept_loop::<campfire_kit::front::serve_plain::{closure}::{closure}, campfire_kit::front::conn::serve_connection<tokio::net::tcp::st` |
| 77.7% | `campfire_kit::front::conn::serve_connection::<tokio::net::tcp::stream::TcpStream>::{closure}` |
| 77.7% | `poll<campfire_kit::front::conn::serve_connection::{async_fn#0}::__tokio_select_util::Out<core::result::Result<(), hyper::error::Error>, (), ()>, campfire_kit::f` |
| 77.7% | `{closure}<tokio::net::tcp::stream::TcpStream>` |
| 77.4% | `<core::pin::Pin<&mut hyper::server::conn::http1::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper::service::util:` |
| 77.4% | `poll<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, axum_core::body::Body, hyper::service::util::ServiceFn<campfire_kit::front::conn::serve` |
| 77.4% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper::service::util::ServiceFn<campfire_kit::front::conn::serve_connection::{async_fn#0}::{closure_env#0}<tokio::` |
| 77.4% | `poll<hyper::proto::h1::dispatch::Server<hyper::service::util::ServiceFn<campfire_kit::front::conn::serve_connection::{async_fn#0}::{closure_env#0}<tokio::net::t` |
| 77.4% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper::service::util::ServiceFn<campfire_kit::front::conn::serve_connection::{async_fn#0}::{closure_env#0}<tokio::` |
| 77.3% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper::service::util::ServiceFn<campfire_kit::front::conn::serve_connection::{async_fn#0}::{closure_env#0}<tokio::n` |
| 73.4% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper::service::util::ServiceFn<campfire_kit::front::conn::serve_connection<tokio::ne` |
| 42.6% | `poll_frame<axum_core::body::Body>` |
| 42.6% | `poll_frame<bytes::bytes::Bytes, axum_core::error::Error>` |
| 42.6% | `axum_core::poll_frame` |
| 42.6% | `<http_body_util::combinators::map_err::MapErr<campfire_kit::front::conn::InFlight<campfire_kit::front::conn::Deadline<axum_core::body::Body>>, <axum_core::error` |
| 42.6% | `poll_frame<campfire_kit::front::conn::Deadline<axum_core::body::Body>>` |
| 42.6% | `campfire_kit::poll_frame` |
| 42.6% | `<http_body_util::combinators::map_err::MapErr<campfire_kit::front::handler::LoggedBody, <axum_core::error::Error>::new<axum_core::error::Error>> as http_body::B` |
| 40.1% | `<http_body_util::combinators::map_err::MapErr<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataSt` |
| 40.1% | `poll_frame<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc::a` |
| 40.1% | `try_poll_next<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc` |
| 40.1% | `poll_next<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc::alloc::Global>>)>, campfire_kit::deflat` |
| 40.1% | `campfire_kit::{async_block#0}` |
| 39.6% | `campfire_kit::compress` |
| 38.1% | `<flate2::mem::Compress as flate2::zio::Ops>::run_vec` |
| 38.1% | `flate2::compress_vec` |
| 38.1% | `flate2::{closure}` |
| 38.1% | `write_to_spare_capacity_of_vec<core::result::Result<flate2::mem::Status, flate2::mem::CompressError>, flate2::mem::{impl#1}::compress_vec::{closure_env#0}>` |
| 38.1% | `flate2::compress_uninit` |
| 38.1% | `compress_uninit<flate2::ffi::zlib_rs::Deflate>` |
| 37.8% | `flate2::compress` |
| 37.8% | `<zlib_rs::stable::Deflate>::compress` |
| 37.8% | `zlib_rs::compress_uninit` |
| 37.7% | `zlib_rs::deflate` |
