# campfire base (native, 4 CPUs): post_message

13904 samples at 1000 µs (13.90 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **sqlite (C)** | (all) | **42.8%** |
| sqlite (C) | syscalls (read/write/epoll/futex) | 12.2% |
| sqlite (C) | malloc/free/realloc | 2.0% |
| sqlite (C) | memcpy/memmove/memset | 0.8% |
| **gzip (miniz_oxide/crc32)** | (all) | **8.6%** |
| gzip (miniz_oxide/crc32) | memcpy/memmove/memset | 0.9% |
| **tokio runtime / scheduling** | (all) | **7.9%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 5.7% |
| **app (controllers/channels/jobs)** | (all) | **6.6%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 3.3% |
| **askama render / view helpers** | (all) | **6.5%** |
| askama render / view helpers | malloc/free/realloc | 3.5% |
| askama render / view helpers | memcpy/memmove/memset | 0.6% |
| **rich text (Action Text pipeline)** | (all) | **6.1%** |
| rich text (Action Text pipeline) | malloc/free/realloc | 2.4% |
| **db models / queries** | (all) | **5.8%** |
| db models / queries | syscalls (read/write/epoll/futex) | 2.1% |
| db models / queries | malloc/free/realloc | 1.4% |
| **kit (request/response plumbing)** | (all) | **5.6%** |
| kit (request/response plumbing) | malloc/free/realloc | 2.2% |
| **json (serde_json)** | (all) | **2.9%** |
| json (serde_json) | malloc/free/realloc | 1.9% |
| **cable** | (all) | **2.7%** |
| **crypto / signing (rails_compat)** | (all) | **2.7%** |
| crypto / signing (rails_compat) | malloc/free/realloc | 1.1% |
| **hyper / http** | (all) | **1.5%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 11.2% | `__syscall_cancel_arch` |
| 5.2% | `syscall` |
| 4.7% | `_int_malloc` |
| 4.3% | `munmap` |
| 2.8% | `miniz_oxide::find_match` |
| 2.4% | `__memcpy_avx512_unaligned_erms` |
| 2.1% | `miniz_oxide::compress_normal` |
| 2.0% | `tcache_get_n` |
| 1.9% | `_int_realloc` |
| 1.8% | `__GI___mmap64` |
| 1.7% | `_int_free_create_chunk` |
| 1.5% | `libsqlite3_sys::sqlite3VdbeExec` |
| 1.4% | `libsqlite3_sys::btreeInitPage` |
| 1.1% | `cfree` |
| 0.9% | `__fcntl64_nocancel_adjusted` |
| 0.8% | `__memcmp_evex_movbe` |
| 0.8% | `__memset_avx512_unaligned_erms` |
| 0.8% | `futex_wait` |
| 0.8% | `unlink_chunk` |
| 0.8% | `format_escaped_str_contents<&mut alloc::vec::Vec<u8, alloc::alloc::Global>, serde_json::ser::CompactFormatter>` |
| 0.8% | `__GI___lll_lock_wake` |
| 0.7% | `next_code_point<core::slice::iter::Iter<u8>>` |
| 0.6% | `alloc::push` |
| 0.5% | `_int_free_chunk` |
| 0.5% | `libsqlite3_sys::sqlite3DbMallocRawNN` |
| 0.5% | `__strlen_evex512` |
| 0.5% | `__libc_realloc` |
| 0.5% | `__libc_malloc2` |
| 0.5% | `lll_mutex_unlock_optimized` |
| 0.5% | `tcache_put_n` |
| 0.5% | `libsqlite3_sys::sqlite3BtreeTableMoveto` |
| 0.4% | `libsqlite3_sys::walChecksumBytes` |
| 0.4% | `miniz_oxide::compress_lz_codes` |
| 0.4% | `__libc_malloc` |
| 0.3% | `campfire_cable::json::escape_html_entities` |
| 0.3% | `_int_free_merge_chunk` |
| 0.3% | `lll_mutex_lock_optimized` |
| 0.3% | `core::eq<u8, u8>` |
| 0.3% | `libsqlite3_sys::yy_reduce.isra.0` |
| 0.3% | `<core::fmt::Arguments>::estimated_capacity` |

## Top inclusive

| incl | function |
|---|---|
| 99.9% | `start_thread` |
| 99.9% | `__GI___clone3` |
| 99.9% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.9% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 81.4% | `tokio::{closure}` |
| 81.4% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 81.4% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 81.4% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 81.4% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 81.4% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 81.4% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 81.4% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 81.4% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 79.0% | `tokio::run` |
| 79.0% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 79.0% | `tokio::poll` |
| 54.3% | `campfire::{closure}` |
| 29.4% | `rusqlite::step` |
| 29.4% | `libsqlite3_sys::sqlite3_step` |
| 29.3% | `libsqlite3_sys::sqlite3Step` |
| 27.2% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 27.2% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 27.2% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 27.2% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 27.2% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 27.2% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 27.2% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 27.2% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 27.2% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 27.2% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 27.2% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 27.2% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 27.2% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 27.2% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 27.2% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 27.2% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 27.2% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 27.2% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 27.2% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 27.0% | `libsqlite3_sys::sqlite3VdbeExec` |
| 26.0% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 25.5% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 25.5% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 25.5% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 24.3% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 24.2% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 24.2% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 24.1% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 24.1% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}` |
| 24.1% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async` |
| 24.1% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 24.1% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 24.1% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
| 24.1% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::se` |
| 24.1% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 24.1% | `campfire_kit::server::serve::<campfire::app::serve::{closure}::{closure}>::{closure}::{closure} (.llvm.1660006695963162447)` |
| 24.1% | `{closure}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>` |
| 24.1% | `<core::future::poll_fn::PollFn<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}::{closure}> as core::future::future` |
| 24.0% | `<hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::TowerToHyp` |
| 24.0% | `poll<&mut hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::T` |
| 24.0% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 24.0% | `poll<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, campfire_` |
| 24.0% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 24.0% | `poll<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>>, axum_core::body::Body, hyper_util::service::glue::T` |
| 23.9% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, camp` |
| 22.7% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages` |
| 22.7% | `{closure}<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::messages:` |
| 22.7% | `poll_future<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::message` |
| 22.7% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::messages` |
| 22.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 22.7% | `catch_unwind<core::task::poll::Poll<core::result::Result<(), campfire_db::error::Error>>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harne` |
| 22.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<camp` |
| 22.7% | `call_once<core::task::poll::Poll<core::result::Result<(), campfire_db::error::Error>>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtim` |
| 22.7% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages::broa` |
| 22.6% | `<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages::broadcast_create::{closure}::{closure` |
| 22.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(),` |
| 22.6% | `{closure}<(), campfire::controllers::messages::broadcast_create::{async_fn#0}::{closure_env#0}>` |
| 22.6% | `<campfire_db::database::ReaderPool>::with::<(), campfire::controllers::messages::broadcast_create::{closure}::{closure}>` |
| 21.3% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<a` |
| 19.5% | `<campfire::controllers::presenters::Presenter>::message` |
