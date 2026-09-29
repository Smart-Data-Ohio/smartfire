# campfire base (native, 4 CPUs): cable1000

37473 samples at 1000 µs (37.47 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **websocket (tungstenite)** | (all) | **84.3%** |
| websocket (tungstenite) | memcpy/memmove/memset | 60.2% |
| websocket (tungstenite) | syscalls (read/write/epoll/futex) | 20.5% |
| websocket (tungstenite) | malloc/free/realloc | 0.8% |
| **cable** | (all) | **8.4%** |
| cable | memcpy/memmove/memset | 1.0% |
| cable | malloc/free/realloc | 0.7% |
| **tokio runtime / scheduling** | (all) | **3.5%** |
| **sqlite (C)** | (all) | **1.8%** |
| **kit (request/response plumbing)** | (all) | **0.4%** |
| **db models / queries** | (all) | **0.4%** |
| **app (controllers/channels/jobs)** | (all) | **0.3%** |
| **rich text (Action Text pipeline)** | (all) | **0.3%** |
| **askama render / view helpers** | (all) | **0.2%** |
| **crypto / signing (rails_compat)** | (all) | **0.2%** |
| **json (serde_json)** | (all) | **0.1%** |
| **hyper / http** | (all) | **0.1%** |
| **gzip (miniz_oxide/crc32)** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 57.3% | `__memset_avx512_unaligned_erms` |
| 20.5% | `__syscall_cancel_arch` |
| 4.0% | `__memcpy_avx512_unaligned_erms` |
| 0.9% | `try_advancing_head<campfire_cable::connection::Control>` |
| 0.8% | `<futures_core::task::__internal::atomic_waker::AtomicWaker>::register` |
| 0.8% | `<futures_util::lock::bilock::BiLock<axum::extract::ws::WebSocket>>::poll_lock (.llvm.4356751827049379152)` |
| 0.7% | `_int_free_chunk` |
| 0.6% | `{async_fn#0}<alloc::string::String>` |
| 0.5% | `replace<core::option::Option<bytes::bytes::Bytes>>` |
| 0.4% | `pop<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 0.4% | `parking_lot::unlock` |
| 0.4% | `tokio::bitand` |
| 0.4% | `try_advancing_head<alloc::string::String>` |
| 0.4% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 0.3% | `_int_malloc` |
| 0.3% | `parking_lot::lock` |
| 0.3% | `get<tokio::sync::mpsc::chan::RxFields<alloc::string::String>>` |
| 0.3% | `<axum::extract::ws::WebSocketUpgrade>::on_upgrade::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, campfire_cable:` |
| 0.3% | `find_block<alloc::string::String>` |
| 0.3% | `core::atomic_load<usize>` |
| 0.3% | `syscall` |
| 0.2% | `<tokio::sync::broadcast::Receiver<()>>::recv_ref (.llvm.10281140562523393653)` |
| 0.2% | `<tokio::sync::mpsc::list::Tx<alloc::string::String>>::push` |
| 0.2% | `write_bytes<core::mem::maybe_uninit::MaybeUninit<u8>>` |
| 0.2% | `core::copy_nonoverlapping<u8>` |
| 0.2% | `<campfire_cable::channel::Subscription<campfire::channels::CableUser>>::stream_from::<alloc::string::String>::{closure} (.llvm.1660006695963162447)` |
| 0.2% | `tokio::poll` |
| 0.2% | `<tokio::sync::broadcast::Receiver<alloc::sync::Arc<str>>>::recv_ref (.llvm.10281140562523393653)` |
| 0.2% | `tokio::macros::support::thread_rng_n` |
| 0.2% | `core::index_mut<u8>` |
| 0.2% | `unwrap<core::ptr::non_null::NonNull<tokio::sync::broadcast::Waiter>>` |
| 0.2% | `tokio::start_index` |
| 0.1% | `munmap` |
| 0.1% | `tokio::project` |
| 0.1% | `_int_realloc` |
| 0.1% | `tcache_get_n` |
| 0.1% | `__libc_realloc` |
| 0.1% | `read<alloc::string::String>` |
| 0.1% | `unlink_chunk` |
| 0.1% | `<core::future::poll_fn::PollFn<campfire_cable::connection::run<campfire::channels::CableUser>::{closure}::{closure}> as core::future::future::Future>::poll` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 100.0% | `start_thread` |
| 100.0% | `__GI___clone3` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.4% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 99.4% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.4% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 99.4% | `tokio::{closure}` |
| 99.4% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.4% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 99.4% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.4% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.4% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.3% | `tokio::run` |
| 99.3% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 99.3% | `tokio::poll` |
| 96.6% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 96.6% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 96.6% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 96.6% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 96.6% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 96.6% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 96.6% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 96.6% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 96.6% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 96.6% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 96.6% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 96.6% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 96.6% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 96.6% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 96.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 96.6% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 96.6% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 96.6% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 96.6% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 96.0% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 96.0% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 96.0% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 95.9% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 90.3% | `<tokio::runtime::task::harness::Harness<<axum::extract::ws::WebSocketUpgrade>::on_upgrade<<campfire_cable::server::Server<campfire::channels::CableUser>>::call:` |
| 90.3% | `poll_inner<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{asyn` |
| 90.3% | `poll_future<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{asy` |
| 90.3% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{async_b` |
| 90.3% | `<tokio::runtime::task::core::Core<<axum::extract::ws::WebSocketUpgrade>::on_upgrade<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{clos` |
| 90.3% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::w` |
| 90.3% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<a` |
| 90.3% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<axum::extract::ws::{impl#1}::on_upgrade::{as` |
| 90.3% | `{closure}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{async` |
| 90.3% | `{closure}<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cable::server::{impl#3}::call::{async` |
| 90.3% | `with_mut<tokio::runtime::task::core::Stage<axum::extract::ws::{impl#1}::on_upgrade::{async_block_env#0}<axum::extract::ws::DefaultOnFailedUpgrade, campfire_cabl` |
| 90.3% | `<axum::extract::ws::WebSocketUpgrade>::on_upgrade::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, campfire_cable:` |
| 90.0% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 83.8% | `{closure}<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 63.7% | `<core::future::poll_fn::PollFn<campfire_cable::connection::run<campfire::channels::CableUser>::{closure}::{closure}> as core::future::future::Future>::poll` |
| 63.6% | `{closure}<campfire::channels::CableUser>` |
| 60.7% | `poll<futures_util::stream::stream::split::SplitStream<axum::extract::ws::WebSocket>>` |
| 60.7% | `<futures_util::stream::stream::split::SplitStream<axum::extract::ws::WebSocket> as futures_core::stream::Stream>::poll_next` |
| 60.7% | `poll_next_unpin<futures_util::stream::stream::split::SplitStream<axum::extract::ws::WebSocket>>` |
| 59.9% | `<axum::extract::ws::WebSocket as futures_core::stream::Stream>::poll_next` |
| 59.9% | `<tokio_tungstenite::WebSocketStream<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>> as futures_core::stream::Stream>::poll_next` |
| 59.9% | `poll_next_unpin<tokio_tungstenite::WebSocketStream<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 59.8% | `with_context<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, tokio_tungstenite::{impl#1}::poll_next::{closure_env#0}<hyper_util::rt::tokio::TokioIo<hy` |
| 59.1% | `<tungstenite::protocol::WebSocketContext>::read::<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 59.1% | `read<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 59.1% | `read_message_frame<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 59.1% | `<tungstenite::protocol::frame::FrameCodec>::read_frame::<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 59.0% | `read_in<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 57.6% | `<bytes::bytes_mut::BytesMut>::resize` |
| 57.6% | `write_bytes<core::mem::maybe_uninit::MaybeUninit<u8>>` |
| 57.3% | `__memset_avx512_unaligned_erms` |
| 25.4% | `{async_fn#0}<campfire::channels::CableUser>` |
| 25.2% | `poll<futures_util::stream::stream::split::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>, axum::extract::ws::Message>` |
| 25.1% | `<futures_util::stream::stream::split::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message> as futures_sink::Sink<axum::extract::ws::Message>>::po` |
| 20.7% | `<axum::extract::ws::WebSocket as futures_sink::Sink<axum::extract::ws::Message>>::poll_flush` |
| 20.7% | `<tokio_tungstenite::WebSocketStream<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>> as futures_sink::Sink<tungstenite::protocol::message::Message>>::p` |
| 20.7% | `with_context<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, tokio_tungstenite::{impl#3}::poll_flush::{closure_env#0}<hyper_util::rt::tokio::TokioIo<h` |
| 20.5% | `internal_syscall_cancel` |
| 20.5% | `syscall_cancel` |
| 20.5% | `__syscall_cancel_arch` |
| 20.5% | `flush<tokio_tungstenite::compat::AllowStd<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
