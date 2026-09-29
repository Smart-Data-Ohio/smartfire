# campfire base (native, 4 CPUs): room_show

24439 samples at 1000 µs (24.44 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **gzip (miniz_oxide/crc32)** | (all) | **61.8%** |
| **sqlite (C)** | (all) | **12.4%** |
| sqlite (C) | malloc/free/realloc | 0.7% |
| **rich text (Action Text pipeline)** | (all) | **11.1%** |
| rich text (Action Text pipeline) | malloc/free/realloc | 3.5% |
| rich text (Action Text pipeline) | memcpy/memmove/memset | 0.6% |
| **crypto / signing (rails_compat)** | (all) | **5.3%** |
| crypto / signing (rails_compat) | malloc/free/realloc | 0.5% |
| **app (controllers/channels/jobs)** | (all) | **2.6%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 1.1% |
| **askama render / view helpers** | (all) | **2.3%** |
| askama render / view helpers | malloc/free/realloc | 1.0% |
| askama render / view helpers | memcpy/memmove/memset | 0.6% |
| **tokio runtime / scheduling** | (all) | **1.2%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 1.1% |
| **db models / queries** | (all) | **1.2%** |
| **json (serde_json)** | (all) | **0.9%** |
| json (serde_json) | malloc/free/realloc | 0.6% |
| **kit (request/response plumbing)** | (all) | **0.8%** |
| **hyper / http** | (all) | **0.3%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 35.2% | `miniz_oxide::find_match` |
| 16.8% | `miniz_oxide::compress_normal` |
| 2.3% | `miniz_oxide::update_hash` |
| 1.6% | `__memcpy_avx512_unaligned_erms` |
| 1.5% | `_int_malloc` |
| 1.1% | `miniz_oxide::compress_lz_codes` |
| 1.1% | `core_arch::_mm_add_epi32` |
| 1.0% | `syscall` |
| 0.9% | `core_arch::_mm_shuffle_epi32<14>` |
| 0.9% | `miniz_oxide::write_code` |
| 0.8% | `miniz_oxide::record_literal` |
| 0.8% | `core_arch::_mm_alignr_epi8<4>` |
| 0.7% | `core_arch::_mm_sha256msg1_epu32` |
| 0.6% | `libsqlite3_sys::sqlite3VdbeExec` |
| 0.6% | `tcache_get_n` |
| 0.6% | `_int_free_create_chunk` |
| 0.6% | `__libc_realloc` |
| 0.6% | `__syscall_cancel_arch` |
| 0.5% | `cfree` |
| 0.4% | `__libc_malloc2` |
| 0.4% | `miniz_oxide::read_unaligned_u64` |
| 0.4% | `__memcmp_evex_movbe` |
| 0.4% | `core_arch::_mm_sha256rnds2_epu32` |
| 0.4% | `miniz_oxide::plant_flag` |
| 0.4% | `miniz_oxide::put_fast` |
| 0.4% | `_int_realloc` |
| 0.4% | `unlink_chunk` |
| 0.4% | `__fcntl64_nocancel_adjusted` |
| 0.4% | `_int_free_chunk` |
| 0.4% | `core::lt` |
| 0.3% | `core::min<usize>` |
| 0.3% | `miniz_oxide::record_match` |
| 0.3% | `libsqlite3_sys::yy_reduce.isra.0` |
| 0.3% | `lll_mutex_unlock_optimized` |
| 0.3% | `core::trailing_zeros` |
| 0.3% | `__libc_malloc` |
| 0.3% | `core::eq<u8, u8>` |
| 0.3% | `_int_free_merge_chunk` |
| 0.3% | `__strlen_evex512` |
| 0.3% | `libsqlite3_sys::sqlite3RunParser` |

## Top inclusive

| incl | function |
|---|---|
| 99.9% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.9% | `start_thread` |
| 99.9% | `__GI___clone3` |
| 99.9% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 99.9% | `tokio::{closure}` |
| 99.9% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 99.9% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.9% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.9% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 99.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.9% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.9% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 99.4% | `tokio::run` |
| 99.4% | `tokio::poll` |
| 99.4% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 71.9% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 71.9% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 71.9% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 71.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 71.9% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 71.9% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 71.9% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 71.9% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 71.9% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 71.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 71.9% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 71.9% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 71.9% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 71.9% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 71.9% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 71.9% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 71.9% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 71.9% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 71.9% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 71.8% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 71.8% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 71.8% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 71.8% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 71.8% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 71.8% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 71.8% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 71.8% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}` |
| 71.8% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async` |
| 71.8% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 71.8% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 71.8% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::se` |
| 71.8% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
| 71.8% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 71.8% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 71.8% | `campfire_kit::server::serve::<campfire::app::serve::{closure}::{closure}>::{closure}::{closure} (.llvm.1660006695963162447)` |
| 71.8% | `{closure}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>` |
| 71.8% | `<core::future::poll_fn::PollFn<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}::{closure}> as core::future::future` |
| 71.8% | `poll<&mut hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::T` |
| 71.8% | `<hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::TowerToHyp` |
| 71.8% | `poll<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, campfire_` |
| 71.8% | `poll<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>>, axum_core::body::Body, hyper_util::service::glue::T` |
| 71.8% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 71.8% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 71.7% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, camp` |
| 71.1% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<a` |
| 61.7% | `poll_frame<bytes::bytes::Bytes, axum_core::error::Error>` |
| 61.7% | `axum_core::poll_frame` |
| 61.7% | `<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec` |
| 61.7% | `<http_body_util::combinators::map_err::MapErr<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataSt` |
| 61.7% | `<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8>>)>, campfire_kit::` |
| 61.7% | `poll_next<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc::alloc::Global>>)>, campfire_kit::deflat` |
| 61.7% | `campfire_kit::{async_block#0}` |
| 61.6% | `campfire_kit::compress` |
| 61.4% | `<flate2::mem::Compress as flate2::zio::Ops>::run_vec` |
| 61.4% | `flate2::compress_vec` |
| 61.4% | `write_to_spare_capacity_of_vec<core::result::Result<flate2::mem::Status, flate2::mem::CompressError>, flate2::mem::{impl#1}::compress_vec::{closure_env#0}>` |
| 61.4% | `flate2::{closure}` |
| 61.4% | `compress_uninit<flate2::ffi::miniz_oxide::Deflate>` |
| 61.4% | `flate2::compress_uninit` |
| 61.4% | `flate2::compress` |
| 61.4% | `miniz_oxide::deflate::stream::deflate` |
| 61.4% | `miniz_oxide::compress` |
| 61.4% | `miniz_oxide::deflate::core::compress_inner` |
| 59.3% | `write<alloc::vec::Vec<u8, alloc::alloc::Global>>` |
