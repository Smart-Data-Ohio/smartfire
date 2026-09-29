# campfire base (native, 4 CPUs): post_message_c1

7482 samples at 1000 µs (7.48 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **sqlite (C)** | (all) | **39.5%** |
| sqlite (C) | syscalls (read/write/epoll/futex) | 9.2% |
| sqlite (C) | malloc/free/realloc | 2.2% |
| sqlite (C) | memcpy/memmove/memset | 1.0% |
| **gzip (miniz_oxide/crc32)** | (all) | **10.1%** |
| gzip (miniz_oxide/crc32) | memcpy/memmove/memset | 1.0% |
| **tokio runtime / scheduling** | (all) | **8.9%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 6.2% |
| **rich text (Action Text pipeline)** | (all) | **6.9%** |
| rich text (Action Text pipeline) | malloc/free/realloc | 2.4% |
| **app (controllers/channels/jobs)** | (all) | **6.5%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 2.7% |
| app (controllers/channels/jobs) | memcpy/memmove/memset | 0.5% |
| **kit (request/response plumbing)** | (all) | **6.4%** |
| kit (request/response plumbing) | malloc/free/realloc | 2.2% |
| **askama render / view helpers** | (all) | **5.9%** |
| askama render / view helpers | malloc/free/realloc | 2.8% |
| askama render / view helpers | memcpy/memmove/memset | 0.5% |
| **db models / queries** | (all) | **4.7%** |
| db models / queries | malloc/free/realloc | 1.5% |
| db models / queries | syscalls (read/write/epoll/futex) | 0.8% |
| **crypto / signing (rails_compat)** | (all) | **3.6%** |
| crypto / signing (rails_compat) | malloc/free/realloc | 1.8% |
| **json (serde_json)** | (all) | **3.0%** |
| json (serde_json) | malloc/free/realloc | 1.8% |
| **cable** | (all) | **2.6%** |
| **hyper / http** | (all) | **2.0%** |
| **other** | (all) | **0.1%** |

## Top self

| self | function |
|---|---|
| 10.8% | `__syscall_cancel_arch` |
| 4.2% | `_int_malloc` |
| 4.1% | `syscall` |
| 2.7% | `miniz_oxide::find_match` |
| 2.6% | `miniz_oxide::compress_normal` |
| 2.6% | `__memcpy_avx512_unaligned_erms` |
| 2.4% | `tcache_get_n` |
| 2.0% | `_int_free_create_chunk` |
| 1.7% | `munmap` |
| 1.7% | `libsqlite3_sys::sqlite3VdbeExec` |
| 1.6% | `_int_realloc` |
| 1.4% | `__fcntl64_nocancel_adjusted` |
| 1.1% | `cfree` |
| 1.1% | `__GI___mmap64` |
| 1.0% | `__memset_avx512_unaligned_erms` |
| 0.8% | `__memcmp_evex_movbe` |
| 0.8% | `libsqlite3_sys::btreeInitPage` |
| 0.7% | `libsqlite3_sys::walChecksumBytes` |
| 0.7% | `format_escaped_str_contents<&mut alloc::vec::Vec<u8, alloc::alloc::Global>, serde_json::ser::CompactFormatter>` |
| 0.6% | `libsqlite3_sys::sqlite3DbMallocRawNN` |
| 0.6% | `next_code_point<core::slice::iter::Iter<u8>>` |
| 0.6% | `_int_free_chunk` |
| 0.5% | `__libc_realloc` |
| 0.5% | `unlink_chunk` |
| 0.5% | `lll_mutex_unlock_optimized` |
| 0.5% | `alloc::push` |
| 0.5% | `miniz_oxide::compress_lz_codes` |
| 0.5% | `__libc_malloc` |
| 0.4% | `tcache_put_n` |
| 0.4% | `<core::fmt::Arguments>::estimated_capacity` |
| 0.4% | `libsqlite3_sys::sqlite3BtreeTableMoveto` |
| 0.4% | `__libc_malloc2` |
| 0.3% | `__strlen_evex512` |
| 0.3% | `campfire_cable::json::escape_html_entities` |
| 0.3% | `core::eq<u8, u8>` |
| 0.3% | `binary_search_by<(&str, &str), campfire_assets::helpers::digested_path::{closure_env#0}>` |
| 0.3% | `libsqlite3_sys::yy_find_shift_action` |
| 0.3% | `libsqlite3_sys::walFindFrame.constprop.0` |
| 0.3% | `miniz_oxide::write_code` |
| 0.3% | `libsqlite3_sys::yy_reduce.isra.0` |

## Top inclusive

| incl | function |
|---|---|
| 99.8% | `start_thread` |
| 99.8% | `__GI___clone3` |
| 99.8% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 99.8% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 79.9% | `tokio::{closure}` |
| 79.9% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 79.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 79.9% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 79.9% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 79.9% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 79.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 79.9% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 79.9% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 78.4% | `tokio::run` |
| 78.4% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 78.4% | `tokio::poll` |
| 51.9% | `campfire::{closure}` |
| 31.1% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 31.1% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 31.1% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 31.1% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 31.1% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 31.1% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 31.1% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 31.1% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 31.1% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 31.1% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 31.1% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 31.1% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 31.1% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 31.1% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 31.1% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 31.1% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 31.1% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 31.1% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 31.1% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 28.5% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 27.6% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 27.6% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 27.6% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 26.7% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 26.6% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 26.5% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 26.5% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 26.5% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 26.5% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 26.5% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}` |
| 26.5% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{async` |
| 26.5% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::se` |
| 26.5% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
| 26.5% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 26.5% | `campfire_kit::server::serve::<campfire::app::serve::{closure}::{closure}>::{closure}::{closure} (.llvm.1660006695963162447)` |
| 26.5% | `{closure}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>` |
| 26.5% | `<core::future::poll_fn::PollFn<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}::{closure}> as core::future::future` |
| 26.3% | `poll<&mut hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::T` |
| 26.3% | `<hyper_util::server::conn::auto::UpgradeableConnection<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>, hyper_util::service::glue::TowerToHyp` |
| 26.3% | `poll<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>>, axum_core::body::Body, hyper_util::service::glue::T` |
| 26.3% | `poll_inner<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 26.3% | `poll_catch<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, cam` |
| 26.3% | `poll<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, campfire_` |
| 26.2% | `poll_loop<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<axum::routing::Router<()>, camp` |
| 24.6% | `rusqlite::step` |
| 24.6% | `libsqlite3_sys::sqlite3_step` |
| 24.6% | `libsqlite3_sys::sqlite3Step` |
| 23.7% | `<hyper::proto::h1::dispatch::Dispatcher<hyper::proto::h1::dispatch::Server<hyper_util::service::glue::TowerToHyperService<tower::util::map_request::MapRequest<a` |
| 22.8% | `libsqlite3_sys::sqlite3VdbeExec` |
| 21.7% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages` |
| 21.7% | `{closure}<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::messages:` |
| 21.7% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::messages` |
| 21.7% | `catch_unwind<core::task::poll::Poll<core::result::Result<(), campfire_db::error::Error>>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harne` |
| 21.7% | `poll_future<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(), campfire::controllers::message` |
| 21.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<camp` |
| 21.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 21.7% | `call_once<core::task::poll::Poll<core::result::Result<(), campfire_db::error::Error>>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtim` |
| 21.7% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages::broa` |
| 21.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<(),` |
| 21.6% | `<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<(), campfire::controllers::messages::broadcast_create::{closure}::{closure` |
| 21.6% | `{closure}<(), campfire::controllers::messages::broadcast_create::{async_fn#0}::{closure_env#0}>` |
| 21.6% | `<campfire_db::database::ReaderPool>::with::<(), campfire::controllers::messages::broadcast_create::{closure}::{closure}>` |
| 19.9% | `campfire_db::{closure}` |
