# campfire main: cable1000

31480 samples at 1000 µs (31.48 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **tokio runtime / scheduling** | (all) | **61.6%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 2.0% |
| **cable** | (all) | **23.4%** |
| cable | malloc/free/realloc | 6.8% |
| **sqlite (C)** | (all) | **6.7%** |
| **app (controllers/channels/jobs)** | (all) | **2.2%** |
| **hyper / http** | (all) | **1.9%** |
| **kit (request/response plumbing)** | (all) | **1.4%** |
| **db models / queries** | (all) | **1.0%** |
| db models / queries | syscalls (read/write/epoll/futex) | 0.5% |
| **rich text (Action Text pipeline)** | (all) | **0.8%** |
| **askama render / view helpers** | (all) | **0.4%** |
| **json (serde_json)** | (all) | **0.2%** |
| **crypto / signing (rails_compat)** | (all) | **0.2%** |
| **gzip (miniz_oxide/crc32)** | (all) | **0.0%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 53.4% | `libc.so.6+0x7795b1aa0952` |
| 3.1% | `try_advancing_head<campfire_cable::socket::Incoming>` |
| 1.8% | `syscall` |
| 1.5% | `<tokio::net::tcp::stream::TcpStream as tokio::io::async_write::AsyncWrite>::poll_write_vectored` |
| 1.4% | `tokio::bitand` |
| 1.4% | `std::lock` |
| 1.3% | `parking_lot::unlock` |
| 1.3% | `poll_next<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream:` |
| 1.3% | `parking_lot::lock` |
| 1.1% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 1.1% | `as_ptr<futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<core::pin::Pin<alloc::boxed::Box<(dyn futures` |
| 1.0% | `<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}` |
| 1.0% | `futures_core::register` |
| 0.8% | `core::atomic_load<u8>` |
| 0.7% | `replace<core::option::Option<campfire_cable::connection::Close>>` |
| 0.6% | `get<tokio::sync::mpsc::chan::RxFields<campfire_cable::socket::Incoming>>` |
| 0.5% | `<tokio::sync::broadcast::Receiver<()>>::recv_ref` |
| 0.5% | `munmap` |
| 0.5% | `tokio::ref_count` |
| 0.5% | `core::clone` |
| 0.4% | `cache_bin_alloc_impl` |
| 0.4% | `libsqlite3_sys::sqlite3VdbeExec` |
| 0.4% | `{async_fn#0}<campfire_cable::socket::Frame>` |
| 0.4% | `pop<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 0.4% | `tokio::ref_inc` |
| 0.3% | `core::atomic_or<usize, usize>` |
| 0.3% | `spin_next_all<futures_util::stream::stream::into_future::StreamFuture<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::Stream<Item=core::result::Resu` |
| 0.3% | `do_register<&core::task::wake::Waker>` |
| 0.3% | `atomic_load_head_and_len_all<futures_util::stream::stream::into_future::StreamFuture<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::Stream<Item=cor` |
| 0.3% | `write<core::option::Option<core::ptr::non_null::NonNull<tokio::sync::notify::Waiter>>>` |
| 0.3% | `sched_yield` |
| 0.3% | `atomic_store<*mut futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable` |
| 0.3% | `drop<campfire_cable::socket::Payload, alloc::alloc::Global>` |
| 0.3% | `as_ptr<futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin:` |
| 0.3% | `clone<campfire_cable::socket::Payload, alloc::alloc::Global>` |
| 0.2% | `atomic_load_head_and_len_all<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn fu` |
| 0.2% | `futures_core::wake` |
| 0.2% | `libsqlite3_sys::btreeInitPage` |
| 0.2% | `dequeue<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::S` |
| 0.2% | `take_value<campfire_cable::pubsub::Subscriber, campfire_cable::pubsub::{impl#2}::deliveries::{closure}::{async_block_env#0}>` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `libc.so.6+0x7795b1b2080b` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 100.0% | `libc.so.6+0x7795b1a980a1` |
| 96.9% | `tokio::{closure}` |
| 96.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 96.9% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 96.9% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 96.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 96.9% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 96.9% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 96.9% | `tokio::run` |
| 96.9% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 96.0% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 96.0% | `tokio::poll` |
| 86.6% | `poll<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blocking:` |
| 86.6% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 86.6% | `enter_runtime<tokio::runtime::scheduler::multi_thread::worker::run::{closure_env#0}, ()>` |
| 86.6% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 86.6% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 86.6% | `set<tokio::runtime::scheduler::Context, tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}, ()>` |
| 86.6% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 86.6% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 86.6% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 86.6% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 86.6% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 86.6% | `poll<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}, ()>` |
| 86.6% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 86.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 86.6% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 86.6% | `tokio::runtime::task::raw::poll::<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 86.6% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 85.4% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 85.1% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 85.1% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 85.0% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 80.9% | `tokio::runtime::task::raw::poll::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, alloc::sync::Arc<tokio::runtime::` |
| 80.8% | `poll_inner<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler:` |
| 80.8% | `poll<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler::multi` |
| 80.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{asy` |
| 80.7% | `poll_future<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler` |
| 80.7% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::` |
| 80.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{async_fn` |
| 80.7% | `with_mut<tokio::runtime::task::core::Stage<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>>, core::task` |
| 80.7% | `{closure}<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler::` |
| 80.7% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_b` |
| 80.7% | `<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}` |
| 79.7% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 57.9% | `{async_fn#0}<campfire::channels::CableUser>` |
| 57.6% | `<campfire_cable::socket::Writer<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>>::send::{closure}` |
| 57.1% | `<campfire_cable::socket::Writer<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>>::write::{closure}` |
| 56.5% | `poll<campfire_cable::socket::{impl#8}::write::{async_fn#0}::{async_block_env#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgra` |
| 56.5% | `{async_block#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 56.2% | `poll<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 56.2% | `{async_fn#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 56.1% | `poll_write_vectored<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 56.1% | `with_lock<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, core::task::poll::Poll<core::result::Result<usize, core::io::error::Error>>, tokio::io::spli` |
| 56.1% | `poll_write_vectored<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 54.9% | `<tokio::net::tcp::stream::TcpStream as tokio::io::async_write::AsyncWrite>::poll_write_vectored` |
| 54.7% | `{closure}<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 54.7% | `poll_write_vectored<hyper::upgrade::Upgraded>` |
| 54.6% | `hyper::poll_write_vectored` |
| 54.6% | `poll_write_vectored<(dyn hyper::upgrade::Io + core::marker::Send)>` |
| 54.6% | `poll_write_vectored<alloc::boxed::Box<(dyn hyper::upgrade::Io + core::marker::Send), alloc::alloc::Global>>` |
| 53.4% | `libc.so.6+0x7795b1aa0952` |
| 53.4% | `poll_write_vectored<mio::net::tcp::stream::TcpStream>` |
| 53.4% | `poll_write_io<usize, tokio::io::poll_evented::{impl#6}::poll_write_vectored::{closure_env#0}<mio::net::tcp::stream::TcpStream>>` |
| 53.4% | `tokio::poll_write_vectored_priv` |
| 53.4% | `poll_io<usize, tokio::io::poll_evented::{impl#6}::poll_write_vectored::{closure_env#0}<mio::net::tcp::stream::TcpStream>>` |
| 51.9% | `mio::{closure}` |
| 51.9% | `<std::sys::net::connection::socket::TcpStream>::write_vectored` |
| 51.9% | `mio::write_vectored` |
| 51.9% | `<std::sys::fd::unix::FileDesc>::write_vectored` |
| 51.9% | `do_io<std::net::tcp::TcpStream, mio::net::tcp::stream::{impl#4}::write_vectored::{closure_env#0}, usize>` |
| 51.9% | `<std::sys::net::connection::socket::unix::Socket>::write_vectored` |
| 51.9% | `<&std::net::tcp::TcpStream as std::io::Write>::write_vectored` |
| 51.9% | `{closure}<mio::net::tcp::stream::TcpStream>` |
| 51.9% | `writev` |
| 12.3% | `{closure}<campfire::channels::CableUser>` |
| 12.3% | `poll<campfire_cable::connection::run::{async_fn#0}::__tokio_select_util::Out<core::option::Option<campfire_cable::socket::Incoming>, core::option::Option<core::` |
