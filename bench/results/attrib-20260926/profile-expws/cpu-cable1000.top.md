# campfire expws: cable1000

34044 samples at 1000 µs (34.04 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **websocket (tungstenite)** | (all) | **62.6%** |
| websocket (tungstenite) | syscalls (read/write/epoll/futex) | 43.5% |
| websocket (tungstenite) | memcpy/memmove/memset | 10.0% |
| websocket (tungstenite) | malloc/free/realloc | 1.9% |
| **cable** | (all) | **19.5%** |
| cable | memcpy/memmove/memset | 2.4% |
| cable | malloc/free/realloc | 1.4% |
| cable | syscalls (read/write/epoll/futex) | 0.9% |
| **tokio runtime / scheduling** | (all) | **8.6%** |
| tokio runtime / scheduling | malloc/free/realloc | 0.9% |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 0.7% |
| **sqlite (C)** | (all) | **4.5%** |
| sqlite (C) | syscalls (read/write/epoll/futex) | 0.6% |
| **kit (request/response plumbing)** | (all) | **1.0%** |
| **db models / queries** | (all) | **0.8%** |
| **rich text (Action Text pipeline)** | (all) | **0.7%** |
| **app (controllers/channels/jobs)** | (all) | **0.7%** |
| **askama render / view helpers** | (all) | **0.5%** |
| **crypto / signing (rails_compat)** | (all) | **0.4%** |
| **hyper / http** | (all) | **0.4%** |
| **json (serde_json)** | (all) | **0.3%** |
| **gzip (miniz_oxide/crc32)** | (all) | **0.0%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 44.1% | `__syscall_cancel_arch` |
| 10.3% | `__memcpy_avx512_unaligned_erms` |
| 2.4% | `__memset_avx512_unaligned_erms` |
| 1.8% | `try_advancing_head<campfire_cable::connection::Control>` |
| 1.7% | `<futures_util::lock::bilock::BiLock<axum::extract::ws::WebSocket>>::poll_lock` |
| 1.7% | `_int_free_chunk` |
| 1.6% | `<futures_core::task::__internal::atomic_waker::AtomicWaker>::register` |
| 1.4% | `replace<core::option::Option<bytes::bytes::Bytes>>` |
| 1.1% | `{async_fn#0}<alloc::string::String>` |
| 1.0% | `parking_lot::unlock` |
| 0.9% | `parking_lot::lock` |
| 0.9% | `try_advancing_head<alloc::string::String>` |
| 0.9% | `tokio::bitand` |
| 0.8% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 0.8% | `pop<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 0.7% | `find_block<alloc::string::String>` |
| 0.6% | `tokio::macros::support::thread_rng_n` |
| 0.6% | `_int_malloc` |
| 0.6% | `core::atomic_load<usize>` |
| 0.6% | `<campfire_cable::channel::Subscription<campfire::channels::CableUser>>::stream_from::<alloc::string::String>::{closure} (.llvm.15186598978220769060)` |
| 0.6% | `<tokio::sync::mpsc::list::Tx<alloc::string::String>>::push` |
| 0.6% | `<tokio::sync::broadcast::Receiver<alloc::sync::Arc<str>>>::recv_ref (.llvm.274245119369245284)` |
| 0.6% | `syscall` |
| 0.5% | `<hyper_util::common::rewind::Rewind<hyper_util::rt::tokio::TokioIo<tokio::net::tcp::stream::TcpStream>> as hyper::rt::io::Read>::poll_read` |
| 0.5% | `<axum::extract::ws::WebSocketUpgrade>::on_upgrade::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, campfire_cable:` |
| 0.5% | `get<tokio::sync::mpsc::chan::RxFields<alloc::string::String>>` |
| 0.5% | `<tokio::sync::broadcast::Receiver<()>>::recv_ref (.llvm.274245119369245284)` |
| 0.4% | `is_some<axum::extract::ws::Message>` |
| 0.4% | `tokio::poll` |
| 0.4% | `read<alloc::string::String>` |
| 0.4% | `tcache_get_n` |
| 0.4% | `tokio::start_index` |
| 0.3% | `_int_realloc` |
| 0.3% | `unwrap<core::ptr::non_null::NonNull<tokio::sync::broadcast::Waiter>>` |
| 0.3% | `core::copy_nonoverlapping<u8>` |
| 0.3% | `tokio::project` |
| 0.3% | `do_register<&core::task::wake::Waker>` |
| 0.2% | `_int_free_create_chunk` |
| 0.2% | `<tokio::sync::broadcast::Shared<alloc::sync::Arc<str>>>::notify_rx (.llvm.14936739848777537204)` |
| 0.2% | `<core::future::poll_fn::PollFn<campfire_cable::connection::run<campfire::channels::CableUser>::{closure}::{closure}> as core::future::future::Future>::poll` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 100.0% | `__GI___clone3` |
| 100.0% | `start_thread` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 98.1% | `tokio::{closure}` |
| 98.1% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 98.1% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 98.1% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 98.1% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 98.1% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 98.1% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 98.1% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 98.1% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 97.9% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 97.9% | `tokio::run` |
| 97.9% | `tokio::poll` |
| 91.6% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 91.6% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 91.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 91.6% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 91.6% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 91.6% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 91.6% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 91.6% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 91.6% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 91.6% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 91.6% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 91.6% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 91.6% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 91.6% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 91.6% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 91.6% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 91.6% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 91.6% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 91.6% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 90.4% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 90.3% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 90.3% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 90.2% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 77.0% | `<tokio::runtime::task::harness::Harness<<axum::extract::ws::WebSocketUpgrade>::on_upgrade<<campfire_cable::server::Server<campfire::channels::CableUser>>::call:` |
| 77.0% | `poll_inner<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{asyn` |
| 76.9% | `poll_future<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{asy` |
| 76.9% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<a` |
| 76.9% | `<tokio::runtime::task::core::Core<<axum::extract::ws::WebSocketUpgrade>::on_upgrade<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{clos` |
| 76.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{as` |
| 76.9% | `{closure}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{async` |
| 76.9% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::w` |
| 76.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{async_b` |
| 76.9% | `with_mut<tokio::runtime::task::core::Stage<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cabl` |
| 76.9% | `{closure}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{async` |
| 76.8% | `<axum::extract::ws::WebSocketUpgrade>::on_upgrade::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, campfire_cable:` |
| 76.3% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 61.9% | `{closure}<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 57.1% | `{async_fn#0}<campfire::channels::CableUser>` |
| 56.5% | `<futures_util::sink::send::Send<futures_util::stream::stream::split::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>, axum::extract::ws::Mes` |
| 56.1% | `<futures_util::stream::stream::split::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message> as futures_sink::Sink<axum::extract::ws::Message>>::po` |
| 44.9% | `<axum::extract::ws::WebSocket as futures_sink::Sink<axum::extract::ws::Message>>::poll_flush` |
| 44.8% | `<tokio_tungstenite::WebSocketStream<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>> as futures_sink::Sink<tungstenite::protocol::message::Message>>::p` |
| 44.8% | `with_context<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, tokio_tungstenite::{impl#3}::poll_flush::{closure_env#0}<hyper_util::rt::tokio::TokioIo<h` |
| 44.3% | `internal_syscall_cancel` |
| 44.3% | `syscall_cancel` |
| 44.1% | `__syscall_cancel_arch` |
| 44.1% | `flush<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 43.9% | `write_out_buffer<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 43.9% | `<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>> as std::io::Write>::write` |
| 43.9% | `with_context<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, tokio_tungstenite::compat::{impl#5}::write::{closure_env#0}<hyper_util::rt::tokio::TokioI` |
| 43.9% | `poll_write<hyper::upgrade::Upgraded>` |
| 43.9% | `<tokio::io::poll_evented::PollEvented<mio::net::tcp::stream::TcpStream>>::poll_write` |
| 43.7% | `mio::{closure}` |
| 43.6% | `<&mio::net::tcp::stream::TcpStream as std::io::Write>::write` |
| 43.6% | `do_io<std::net::tcp::TcpStream, mio::net::tcp::stream::{impl#4}::write::{closure_env#0}, usize>` |
| 43.6% | `<&std::os::unix::net::stream::UnixStream as std::io::Write>::write` |
| 43.6% | `<std::sys::net::connection::socket::TcpStream>::write` |
| 43.6% | `__send` |
| 17.1% | `<core::future::poll_fn::PollFn<campfire_cable::connection::run<campfire::channels::CableUser>::{closure}::{closure}> as core::future::future::Future>::poll` |
| 16.8% | `{closure}<campfire::channels::CableUser>` |
| 10.9% | `poll_flush_slot<axum::extract::ws::WebSocket, axum::extract::ws::Message>` |
| 10.8% | `<axum::extract::ws::WebSocket as futures_sink::Sink<axum::extract::ws::Message>>::start_send` |
| 10.8% | `<tokio_tungstenite::WebSocketStream<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>> as futures_sink::Sink<tungstenite::protocol::message::Message>>::s` |
| 10.7% | `with_context<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, tokio_tungstenite::{impl#3}::start_send::{closure_env#0}<hyper_util::rt::tokio::TokioIo<h` |
