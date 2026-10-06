# campfire base (native, 4 CPUs): messages_page

23833 samples at 1000 µs (23.83 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **gzip (miniz_oxide/crc32)** | (all) | **57.0%** |
| **sqlite (C)** | (all) | **16.2%** |
| sqlite (C) | malloc/free/realloc | 1.0% |
| sqlite (C) | syscalls (read/write/epoll/futex) | 0.7% |
| **rich text (Action Text pipeline)** | (all) | **14.4%** |
| rich text (Action Text pipeline) | malloc/free/realloc | 4.3% |
| rich text (Action Text pipeline) | memcpy/memmove/memset | 0.8% |
| **app (controllers/channels/jobs)** | (all) | **2.8%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 1.2% |
| **askama render / view helpers** | (all) | **2.0%** |
| askama render / view helpers | malloc/free/realloc | 0.8% |
| **tokio runtime / scheduling** | (all) | **1.7%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 1.4% |
| **db models / queries** | (all) | **1.7%** |
| db models / queries | syscalls (read/write/epoll/futex) | 0.6% |
| **crypto / signing (rails_compat)** | (all) | **1.5%** |
| **json (serde_json)** | (all) | **1.3%** |
| json (serde_json) | malloc/free/realloc | 0.9% |
| **kit (request/response plumbing)** | (all) | **1.0%** |
| **hyper / http** | (all) | **0.4%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 29.9% | `miniz_oxide::find_match` |
| 17.1% | `miniz_oxide::compress_normal` |
| 2.3% | `miniz_oxide::update_hash` |
| 1.9% | `__memcpy_avx512_unaligned_erms` |
| 1.9% | `_int_malloc` |
| 1.4% | `syscall` |
| 1.0% | `miniz_oxide::write_code` |
| 0.9% | `miniz_oxide::compress_lz_codes` |
| 0.8% | `miniz_oxide::record_literal` |
| 0.8% | `__syscall_cancel_arch` |
| 0.7% | `cfree` |
| 0.7% | `tcache_get_n` |
| 0.7% | `libsqlite3_sys::sqlite3VdbeExec` |
| 0.7% | `_int_free_create_chunk` |
| 0.6% | `__fcntl64_nocancel_adjusted` |
| 0.5% | `__libc_realloc` |
| 0.5% | `_int_free_chunk` |
| 0.5% | `libsqlite3_sys::yy_reduce.isra.0` |
| 0.5% | `_int_realloc` |
| 0.5% | `miniz_oxide::read_unaligned_u64` |
| 0.5% | `unlink_chunk` |
| 0.5% | `libsqlite3_sys::sqlite3RunParser` |
| 0.4% | `core::eq<u8, u8>` |
| 0.4% | `__libc_malloc2` |
| 0.4% | `miniz_oxide::plant_flag` |
| 0.4% | `miniz_oxide::put_fast` |
| 0.4% | `core::lt` |
| 0.4% | `lll_mutex_unlock_optimized` |
| 0.4% | `core::eq<&str>` |
| 0.4% | `core::min<usize>` |
| 0.4% | `__libc_malloc` |
| 0.3% | `__memset_avx512_unaligned_erms` |
| 0.3% | `__memcmp_evex_movbe` |
| 0.3% | `libsqlite3_sys::yy_find_shift_action` |
| 0.3% | `__strlen_evex512` |
| 0.3% | `libsqlite3_sys::sqlite3DbMallocRawNN` |
| 0.3% | `_int_free_merge_chunk` |
| 0.3% | `__GI___lll_lock_wake` |
| 0.3% | `std::unlock` |
| 0.3% | `write<campfire_richtext::dom::Node>` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `__GI___clone3` |
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 100.0% | `start_thread` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.8% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 99.8% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.8% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.8% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.8% | `tokio::{closure}` |
| 99.8% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 99.8% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 99.8% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.8% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.2% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 99.2% | `tokio::run` |
| 99.2% | `tokio::poll` |
| 63.7% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 63.7% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 63.7% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 63.7% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 63.7% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 63.7% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 63.7% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 63.7% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 63.7% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 63.7% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 63.7% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 63.7% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 63.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 63.7% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 63.7% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 63.7% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 63.7% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 63.7% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 63.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 63.6% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 63.5% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 63.5% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 63.5% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 63.5% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 63.5% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 63.5% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 63.5% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 63.5% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 63.5% | `campfire_kit::server::serve::<campfire::app::serve::{closure}::{closure}>::{closure}::{closure} (.llvm.1660006695963162447)` |
| 63.5% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
| 63.5% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 63.5% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async` |
| 63.5% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}` |
| 63.5% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::se` |
| 63.5% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 63.5% | `<core::future::poll_fn::PollFn<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}::{closure}> as core::future::future` |
| 63.5% | `{closure}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>` |
| 63.5% | `poll<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, campfire_` |
| 63.5% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 63.5% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 63.5% | `<hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::TowerToHyp` |
| 63.5% | `poll<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>>, axum_core::body::Body, hyper_util::service::glue::T` |
| 63.5% | `poll<&mut hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::T` |
| 63.4% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, camp` |
| 62.6% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<a` |
| 56.8% | `poll_frame<bytes::bytes::Bytes, axum_core::error::Error>` |
| 56.8% | `axum_core::poll_frame` |
| 56.8% | `<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec` |
| 56.8% | `<http_body_util::combinators::map_err::MapErr<axum_core::body::StreamBody<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataSt` |
| 56.8% | `poll_next<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8, alloc::alloc::Global>>)>, campfire_kit::deflat` |
| 56.8% | `<futures_util::stream::unfold::Unfold<core::option::Option<(axum_core::body::BodyDataStream, flate2::gz::write::GzEncoder<alloc::vec::Vec<u8>>)>, campfire_kit::` |
| 56.8% | `campfire_kit::{async_block#0}` |
| 56.7% | `campfire_kit::compress` |
| 56.5% | `<flate2::mem::Compress as flate2::zio::Ops>::run_vec` |
| 56.5% | `flate2::compress_vec` |
| 56.5% | `write_to_spare_capacity_of_vec<core::result::Result<flate2::mem::Status, flate2::mem::CompressError>, flate2::mem::{impl#1}::compress_vec::{closure_env#0}>` |
| 56.5% | `flate2::{closure}` |
| 56.5% | `compress_uninit<flate2::ffi::miniz_oxide::Deflate>` |
| 56.5% | `flate2::compress_uninit` |
| 56.4% | `flate2::compress` |
| 56.4% | `miniz_oxide::deflate::core::compress_inner` |
| 56.4% | `miniz_oxide::compress` |
| 56.4% | `miniz_oxide::deflate::stream::deflate` |
| 55.0% | `write<alloc::vec::Vec<u8, alloc::alloc::Global>>` |
