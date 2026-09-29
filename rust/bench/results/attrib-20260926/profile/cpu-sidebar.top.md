# campfire base (native, 4 CPUs): sidebar

20766 samples at 1000 µs (20.77 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **gzip (miniz_oxide/crc32)** | (all) | **39.3%** |
| gzip (miniz_oxide/crc32) | memcpy/memmove/memset | 0.8% |
| **sqlite (C)** | (all) | **18.7%** |
| sqlite (C) | malloc/free/realloc | 1.5% |
| sqlite (C) | syscalls (read/write/epoll/futex) | 1.3% |
| **askama render / view helpers** | (all) | **11.9%** |
| askama render / view helpers | malloc/free/realloc | 5.6% |
| askama render / view helpers | memcpy/memmove/memset | 1.3% |
| **app (controllers/channels/jobs)** | (all) | **8.6%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 3.0% |
| app (controllers/channels/jobs) | memcpy/memmove/memset | 1.1% |
| **crypto / signing (rails_compat)** | (all) | **5.9%** |
| crypto / signing (rails_compat) | malloc/free/realloc | 1.4% |
| **tokio runtime / scheduling** | (all) | **5.5%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 4.7% |
| **kit (request/response plumbing)** | (all) | **4.1%** |
| kit (request/response plumbing) | malloc/free/realloc | 1.4% |
| **db models / queries** | (all) | **3.5%** |
| db models / queries | syscalls (read/write/epoll/futex) | 1.6% |
| db models / queries | malloc/free/realloc | 0.7% |
| **hyper / http** | (all) | **1.5%** |
| **json (serde_json)** | (all) | **1.0%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 19.5% | `miniz_oxide::find_match` |
| 9.6% | `miniz_oxide::compress_normal` |
| 3.4% | `syscall` |
| 3.1% | `__syscall_cancel_arch` |
| 2.7% | `__memcpy_avx512_unaligned_erms` |
| 2.7% | `_int_malloc` |
| 1.9% | `miniz_oxide::compress_lz_codes` |
| 1.4% | `libsqlite3_sys::sqlite3VdbeExec` |
| 1.2% | `_int_free_create_chunk` |
| 1.2% | `__libc_realloc` |
| 1.1% | `cfree` |
| 1.0% | `tcache_get_n` |
| 0.9% | `miniz_oxide::update_hash` |
| 0.9% | `_int_free_chunk` |
| 0.9% | `__memcmp_evex_movbe` |
| 0.8% | `__memset_avx512_unaligned_erms` |
| 0.7% | `futex_wait` |
| 0.7% | `__libc_malloc2` |
| 0.7% | `_int_realloc` |
| 0.6% | `__GI___lll_lock_wake` |
| 0.6% | `core_arch::_mm_add_epi32` |
| 0.6% | `miniz_oxide::put_fast` |
| 0.5% | `__strlen_evex512` |
| 0.5% | `miniz_oxide::write_code` |
| 0.5% | `miniz_oxide::record_match` |
| 0.5% | `_int_free_merge_chunk` |
| 0.5% | `campfire_views::helpers::html::push_escaped` |
| 0.5% | `libsqlite3_sys::columnName` |
| 0.5% | `core_arch::_mm_shuffle_epi32<14>` |
| 0.5% | `__libc_malloc` |
| 0.5% | `<core::fmt::Arguments>::estimated_capacity` |
| 0.4% | `unlink_chunk` |
| 0.4% | `alloc::fmt::format::format_inner` |
| 0.4% | `core_arch::_mm_alignr_epi8<4>` |
| 0.4% | `core::trailing_zeros` |
| 0.4% | `core::fmt::write` |
| 0.4% | `lll_mutex_unlock_optimized` |
| 0.4% | `core_arch::_mm_sha256msg1_epu32` |
| 0.4% | `__rustc::__rust_no_alloc_shim_is_unstable_v2` |
| 0.4% | `__lll_lock_wake_private` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 100.0% | `start_thread` |
| 100.0% | `__GI___clone3` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.3% | `tokio::{closure}` |
| 99.3% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.3% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.3% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.3% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.3% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 99.3% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 99.3% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.3% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 97.9% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 97.9% | `tokio::run` |
| 97.9% | `tokio::poll` |
| 75.4% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 75.4% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 75.4% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 75.4% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 75.4% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 75.4% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 75.4% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 75.4% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 75.4% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 75.4% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 75.4% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 75.4% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 75.4% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 75.4% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 75.4% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 75.4% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 75.4% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 75.4% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 75.4% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 75.1% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 75.1% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 75.1% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 75.0% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 75.0% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 75.0% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 75.0% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 75.0% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async` |
| 75.0% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}` |
| 75.0% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
| 75.0% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 75.0% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 75.0% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 75.0% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::se` |
| 75.0% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 75.0% | `campfire_kit::server::serve::<campfire::app::serve::{closure}::{closure}>::{closure}::{closure} (.llvm.1660006695963162447)` |
| 74.9% | `{closure}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>` |
| 74.9% | `<core::future::poll_fn::PollFn<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}::{closure}> as core::future::future` |
| 74.8% | `poll<&mut hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::T` |
| 74.8% | `<hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::TowerToHyp` |
| 74.8% | `poll<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>>, axum_core::body::Body, hyper_util::service::glue::T` |
| 74.8% | `poll<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, campfire_` |
| 74.8% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 74.8% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 74.8% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, camp` |
| 71.3% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<a` |
| 38.4% | `axum_core::poll_frame` |
| 38.4% | `poll_frame<bytes::bytes::Bytes, axum_core::error::Error>` |
| 38.3% | `<http_body_util::combinators::map_err::MapErr<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataSt` |
| 38.3% | `<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec` |
| 38.3% | `<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8>>)>, campfire_kit::` |
| 38.3% | `poll_next<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc::alloc::Global>>)>, campfire_kit::deflat` |
| 38.3% | `campfire_kit::{async_block#0}` |
| 38.0% | `<flate2::mem::Compress as flate2::zio::Ops>::run_vec` |
| 38.0% | `flate2::compress_vec` |
| 38.0% | `write_to_spare_capacity_of_vec<core::result::Result<flate2::mem::Status, flate2::mem::CompressError>, flate2::mem::{impl#1}::compress_vec::{closure_env#0}>` |
| 38.0% | `flate2::{closure}` |
| 38.0% | `compress_uninit<flate2::ffi::miniz_oxide::Deflate>` |
| 38.0% | `flate2::compress_uninit` |
| 38.0% | `campfire_kit::compress` |
| 37.8% | `flate2::compress` |
| 37.8% | `miniz_oxide::deflate::stream::deflate` |
| 37.8% | `miniz_oxide::compress` |
| 37.8% | `miniz_oxide::deflate::core::compress_inner` |
| 33.9% | `write<alloc::vec::Vec<u8, alloc::alloc::Global>>` |
