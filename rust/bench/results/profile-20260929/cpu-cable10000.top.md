# campfire main: cable10000

31163 samples at 1000 µs (31.16 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **tokio runtime / scheduling** | (all) | **68.8%** |
| tokio runtime / scheduling | malloc/free/realloc | 1.3% |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 0.8% |
| **cable** | (all) | **26.7%** |
| cable | malloc/free/realloc | 9.0% |
| cable | syscalls (read/write/epoll/futex) | 0.7% |
| **hyper / http** | (all) | **1.4%** |
| **sqlite (C)** | (all) | **1.4%** |
| **app (controllers/channels/jobs)** | (all) | **0.7%** |
| **kit (request/response plumbing)** | (all) | **0.3%** |
| **rich text (Action Text pipeline)** | (all) | **0.2%** |
| **db models / queries** | (all) | **0.2%** |
| **askama render / view helpers** | (all) | **0.1%** |
| **crypto / signing (rails_compat)** | (all) | **0.1%** |
| **json (serde_json)** | (all) | **0.0%** |
| **gzip (miniz_oxide/crc32)** | (all) | **0.0%** |
| **other** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 61.1% | `libc.so.6+0x7795b1aa0952` |
| 3.0% | `try_advancing_head<campfire_cable::socket::Incoming>` |
| 1.6% | `poll_next<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream:` |
| 1.3% | `unwrap<core::ptr::non_null::NonNull<tokio::sync::broadcast::Waiter>>` |
| 1.3% | `<tokio::net::tcp::stream::TcpStream as tokio::io::async_write::AsyncWrite>::poll_write_vectored` |
| 1.3% | `as_ptr<futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<core::pin::Pin<alloc::boxed::Box<(dyn futures` |
| 1.3% | `tokio::bitand` |
| 1.3% | `futures_core::wake` |
| 1.3% | `pop<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 1.2% | `std::lock` |
| 1.2% | `parking_lot::unlock` |
| 1.1% | `dequeue<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::S` |
| 0.9% | `as_ptr<futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin:` |
| 0.9% | `futures_core::register` |
| 0.9% | `parking_lot::lock` |
| 0.9% | `core::atomic_or<usize, usize>` |
| 0.7% | `tokio::poll` |
| 0.6% | `syscall` |
| 0.6% | `core::checked_increment` |
| 0.5% | `get<tokio::sync::mpsc::chan::RxFields<campfire_cable::socket::Incoming>>` |
| 0.5% | `replace<core::option::Option<campfire_cable::connection::Close>>` |
| 0.5% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 0.4% | `{async_fn#0}<campfire_cable::socket::Frame>` |
| 0.4% | `sched_yield` |
| 0.4% | `<tokio::sync::broadcast::Receiver<()>>::recv_ref` |
| 0.4% | `clone<campfire_cable::socket::Payload, alloc::alloc::Global>` |
| 0.4% | `drop_glue<tokio::sync::watch::changed_impl::{async_fn_env#0}<campfire_cable::socket::Frame>>` |
| 0.4% | `<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}` |
| 0.3% | `core::atomic_load<u8>` |
| 0.3% | `core::clone` |
| 0.3% | `drop<campfire_cable::socket::Payload, alloc::alloc::Global>` |
| 0.3% | `atomic_store<*mut futures_util::stream::futures_unordered::task::Task<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable` |
| 0.3% | `<futures_util::stream::futures_unordered::FuturesUnordered<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin:` |
| 0.3% | `tokio::ref_count` |
| 0.2% | `atomic_load_head_and_len_all<futures_util::stream::stream::into_future::StreamFuture<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::Stream<Item=cor` |
| 0.2% | `link<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn futures_core::stream::Stre` |
| 0.2% | `atomic_load_head_and_len_all<futures_util::stream::stream::into_future::StreamFuture<futures_util::abortable::Abortable<core::pin::Pin<alloc::boxed::Box<(dyn fu` |
| 0.2% | `<tokio::sync::broadcast::Receiver<campfire_cable::socket::Frame>>::recv_ref` |
| 0.2% | `cooperative<tokio::sync::broadcast::Recv<campfire_cable::socket::Frame>>` |
| 0.2% | `{async_block#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |

## Top inclusive

| incl | function |
|---|---|
| 100.0% | `libc.so.6+0x7795b1b2080b` |
| 100.0% | `libc.so.6+0x7795b1a980a1` |
| 100.0% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 100.0% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 99.3% | `tokio::{closure}` |
| 99.3% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 99.3% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.3% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 99.3% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.3% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.3% | `tokio::run` |
| 99.3% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.3% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.2% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 99.2% | `tokio::poll` |
| 92.6% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 92.6% | `poll<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blocking:` |
| 92.6% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 92.6% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 92.6% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 92.6% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 92.6% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 92.6% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 92.6% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 92.6% | `poll<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}, ()>` |
| 92.6% | `enter_runtime<tokio::runtime::scheduler::multi_thread::worker::run::{closure_env#0}, ()>` |
| 92.6% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 92.6% | `tokio::runtime::task::raw::poll::<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 92.6% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 92.6% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 92.6% | `set<tokio::runtime::scheduler::Context, tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}, ()>` |
| 92.6% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 90.9% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 90.9% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 90.9% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 90.8% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 89.0% | `tokio::runtime::task::raw::poll::<<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}, alloc::sync::Arc<tokio::runtime::` |
| 89.0% | `poll_inner<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler:` |
| 89.0% | `poll<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler::multi` |
| 88.9% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{asy` |
| 88.9% | `{closure}<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler::` |
| 88.9% | `poll_future<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>, alloc::sync::Arc<tokio::runtime::scheduler` |
| 88.9% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::` |
| 88.9% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_b` |
| 88.9% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_cable::server::{impl#3}::call::{async_fn` |
| 88.9% | `with_mut<tokio::runtime::task::core::Stage<campfire_cable::server::{impl#3}::call::{async_fn#0}::{async_block_env#0}<campfire::channels::CableUser>>, core::task` |
| 88.9% | `<campfire_cable::server::Server<campfire::channels::CableUser>>::call::{closure}::{closure}` |
| 88.5% | `campfire_cable::connection::run::<campfire::channels::CableUser>::{closure}` |
| 66.5% | `{async_fn#0}<campfire::channels::CableUser>` |
| 66.0% | `<campfire_cable::socket::Writer<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>>::send::{closure}` |
| 65.7% | `<campfire_cable::socket::Writer<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>>::write::{closure}` |
| 65.2% | `poll<campfire_cable::socket::{impl#8}::write::{async_fn#0}::{async_block_env#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgra` |
| 65.2% | `{async_block#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 65.0% | `poll<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 65.0% | `{async_fn#0}<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 64.9% | `with_lock<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>, core::task::poll::Poll<core::result::Result<usize, core::io::error::Error>>, tokio::io::spli` |
| 64.9% | `poll_write_vectored<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 64.9% | `poll_write_vectored<tokio::io::split::WriteHalf<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>>` |
| 63.6% | `{closure}<hyper_util::rt::tokio::TokioIo<hyper::upgrade::Upgraded>>` |
| 63.6% | `<tokio::net::tcp::stream::TcpStream as tokio::io::async_write::AsyncWrite>::poll_write_vectored` |
| 63.6% | `poll_write_vectored<hyper::upgrade::Upgraded>` |
| 63.6% | `poll_write_vectored<(dyn hyper::upgrade::Io + core::marker::Send)>` |
| 63.6% | `hyper::poll_write_vectored` |
| 63.6% | `poll_write_vectored<alloc::boxed::Box<(dyn hyper::upgrade::Io + core::marker::Send), alloc::alloc::Global>>` |
| 62.3% | `poll_write_vectored<mio::net::tcp::stream::TcpStream>` |
| 62.3% | `poll_io<usize, tokio::io::poll_evented::{impl#6}::poll_write_vectored::{closure_env#0}<mio::net::tcp::stream::TcpStream>>` |
| 62.3% | `tokio::poll_write_vectored_priv` |
| 62.3% | `poll_write_io<usize, tokio::io::poll_evented::{impl#6}::poll_write_vectored::{closure_env#0}<mio::net::tcp::stream::TcpStream>>` |
| 61.1% | `libc.so.6+0x7795b1aa0952` |
| 61.0% | `mio::{closure}` |
| 61.0% | `<std::sys::net::connection::socket::TcpStream>::write_vectored` |
| 61.0% | `mio::write_vectored` |
| 61.0% | `{closure}<mio::net::tcp::stream::TcpStream>` |
| 61.0% | `do_io<std::net::tcp::TcpStream, mio::net::tcp::stream::{impl#4}::write_vectored::{closure_env#0}, usize>` |
| 61.0% | `<&std::net::tcp::TcpStream as std::io::Write>::write_vectored` |
| 61.0% | `<std::sys::fd::unix::FileDesc>::write_vectored` |
| 61.0% | `<std::sys::net::connection::socket::unix::Socket>::write_vectored` |
| 61.0% | `writev` |
| 12.2% | `poll<campfire_cable::connection::run::{async_fn#0}::__tokio_select_util::Out<core::option::Option<campfire_cable::socket::Incoming>, core::option::Option<core::` |
| 12.2% | `{closure}<campfire::channels::CableUser>` |
