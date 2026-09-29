# campfire base (native, 4 CPUs): room_show_identity

22228 samples at 1000 µs (22.23 CPU-s)

## By owner (nearest recognizable frame) and leaf kind

| owner | leaf | share |
|---|---|---|
| **sqlite (C)** | (all) | **32.1%** |
| sqlite (C) | malloc/free/realloc | 1.9% |
| sqlite (C) | syscalls (read/write/epoll/futex) | 1.8% |
| sqlite (C) | memcpy/memmove/memset | 0.8% |
| **rich text (Action Text pipeline)** | (all) | **30.6%** |
| rich text (Action Text pipeline) | malloc/free/realloc | 9.5% |
| rich text (Action Text pipeline) | memcpy/memmove/memset | 1.5% |
| rich text (Action Text pipeline) | syscalls (read/write/epoll/futex) | 0.8% |
| **crypto / signing (rails_compat)** | (all) | **13.2%** |
| crypto / signing (rails_compat) | malloc/free/realloc | 1.1% |
| **tokio runtime / scheduling** | (all) | **5.8%** |
| tokio runtime / scheduling | syscalls (read/write/epoll/futex) | 5.3% |
| **app (controllers/channels/jobs)** | (all) | **5.7%** |
| app (controllers/channels/jobs) | malloc/free/realloc | 2.4% |
| app (controllers/channels/jobs) | memcpy/memmove/memset | 0.5% |
| **askama render / view helpers** | (all) | **4.9%** |
| askama render / view helpers | malloc/free/realloc | 1.8% |
| askama render / view helpers | memcpy/memmove/memset | 1.1% |
| **db models / queries** | (all) | **2.7%** |
| db models / queries | syscalls (read/write/epoll/futex) | 0.9% |
| db models / queries | malloc/free/realloc | 0.6% |
| **json (serde_json)** | (all) | **2.4%** |
| json (serde_json) | malloc/free/realloc | 1.7% |
| **kit (request/response plumbing)** | (all) | **1.9%** |
| kit (request/response plumbing) | malloc/free/realloc | 0.7% |
| **hyper / http** | (all) | **0.5%** |
| **other** | (all) | **0.1%** |
| **gzip (miniz_oxide/crc32)** | (all) | **0.0%** |

## Top self

| self | function |
|---|---|
| 4.1% | `__syscall_cancel_arch` |
| 3.8% | `_int_malloc` |
| 3.5% | `__memcpy_avx512_unaligned_erms` |
| 2.9% | `core_arch::_mm_add_epi32` |
| 2.4% | `syscall` |
| 2.3% | `core_arch::_mm_shuffle_epi32<14>` |
| 2.0% | `core_arch::_mm_alignr_epi8<4>` |
| 1.6% | `libsqlite3_sys::sqlite3VdbeExec` |
| 1.5% | `core_arch::_mm_sha256msg1_epu32` |
| 1.4% | `_int_free_create_chunk` |
| 1.4% | `tcache_get_n` |
| 1.4% | `cfree` |
| 1.4% | `__libc_realloc` |
| 1.1% | `_int_free_chunk` |
| 1.0% | `__libc_malloc2` |
| 1.0% | `libsqlite3_sys::yy_reduce.isra.0` |
| 0.9% | `lll_mutex_unlock_optimized` |
| 0.9% | `_int_realloc` |
| 0.8% | `core_arch::_mm_sha256rnds2_epu32` |
| 0.8% | `__libc_malloc` |
| 0.8% | `__memcmp_evex_movbe` |
| 0.8% | `__GI___lll_lock_wake` |
| 0.8% | `core::eq<u8, u8>` |
| 0.8% | `unlink_chunk` |
| 0.8% | `libsqlite3_sys::sqlite3RunParser` |
| 0.8% | `futex_wait` |
| 0.7% | `libsqlite3_sys::yy_find_shift_action` |
| 0.7% | `__strlen_evex512` |
| 0.7% | `campfire_richtext::escape_text` |
| 0.7% | `core::eq<&str>` |
| 0.6% | `_int_free_merge_chunk` |
| 0.6% | `lll_mutex_lock_optimized` |
| 0.6% | `write<campfire_richtext::dom::Node>` |
| 0.6% | `__rustc::__rust_no_alloc_shim_is_unstable_v2` |
| 0.6% | `__fcntl64_nocancel_adjusted` |
| 0.6% | `std::unlock` |
| 0.5% | `libsqlite3_sys::sqlite3DbMallocRawNN` |
| 0.5% | `core_arch::_mm_sha256msg2_epu32` |
| 0.5% | `<html5ever::tokenizer::Tokenizer<html5ever::tree_builder::TreeBuilder<usize, campfire_richtext::dom::Sink>>>::step` |
| 0.4% | `<html5ever::tree_builder::TreeBuilder<usize, campfire_richtext::dom::Sink>>::step` |

## Top inclusive

| incl | function |
|---|---|
| 99.9% | `__GI___clone3` |
| 99.9% | `<std::sys::thread::unix::Thread>::new::thread_start` |
| 99.9% | `<alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output = ()> + core::marker::Send> as core::ops::function::FnOnce<()>>::call_once` |
| 99.9% | `start_thread` |
| 99.7% | `tokio::{closure}` |
| 99.7% | `call_once<(), std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>>` |
| 99.7% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{imp` |
| 99.7% | `catch_unwind<(), core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::` |
| 99.7% | `std::sys::backtrace::__rust_begin_short_backtrace::<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>` |
| 99.7% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::lifecycle::spawn_unchecked::{closure}::{closure_env#0}<tokio::runtime::blocking::pool::{impl#6}:` |
| 99.7% | `<tokio::runtime::blocking::pool::Inner>::run` |
| 99.7% | `{closure}<tokio::runtime::blocking::pool::{impl#6}::spawn_thread::{closure_env#0}, ()>` |
| 99.7% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure}, ()>::{closure} as core::ops::function::FnOnce<()>>:` |
| 98.8% | `tokio::run` |
| 98.8% | `run<tokio::runtime::blocking::schedule::BlockingSchedule>` |
| 98.8% | `tokio::poll` |
| 75.9% | `campfire::{closure}` |
| 70.5% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<campfire_views::rooms::ShowView, ca` |
| 70.5% | `{closure}<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<campfire_views::rooms::ShowView, cam` |
| 70.4% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<campfire_views::rooms::ShowView, ca` |
| 70.4% | `poll_future<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<campfire_views::rooms::ShowView, c` |
| 70.4% | `catch_unwind<core::task::poll::Poll<core::result::Result<campfire_views::rooms::ShowView, campfire_db::error::Error>>, core::panic::unwind_safe::AssertUnwindSaf` |
| 70.4% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<camp` |
| 70.4% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 70.4% | `call_once<core::task::poll::Poll<core::result::Result<campfire_views::rooms::ShowView, campfire_db::error::Error>>, tokio::runtime::task::harness::poll_future::` |
| 70.4% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<campfire_views::rooms::ShowView, campfire` |
| 70.4% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<campfire_db::database::{impl#4}::read::{async_fn#0}::{closure_env#0}<cam` |
| 70.4% | `<tokio::runtime::blocking::task::BlockingTask<<campfire_db::database::Database>::read<campfire_views::rooms::ShowView, campfire::controllers::rooms::render_show` |
| 70.3% | `{closure}<campfire_views::rooms::ShowView, campfire::controllers::rooms::render_show::{async_fn#0}::{closure_env#0}>` |
| 70.3% | `<campfire_db::database::ReaderPool>::with::<campfire_views::rooms::ShowView, campfire::controllers::rooms::render_show::{closure}::{closure}>` |
| 66.5% | `campfire::messages` |
| 66.5% | `from_iter<campfire_views::messages::MessageView, core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::` |
| 66.5% | `collect<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, campfire::controllers::presenters::{impl#0}::messages::{c` |
| 66.5% | `collect<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, campfire::controllers:` |
| 66.5% | `from_iter<campfire_views::messages::MessageView, campfire_db::error::Error, alloc::vec::Vec<campfire_views::messages::MessageView, alloc::alloc::Global>, core::` |
| 66.5% | `{closure}<campfire_views::messages::MessageView, campfire_db::error::Error, alloc::vec::Vec<campfire_views::messages::MessageView, alloc::alloc::Global>, core::` |
| 66.5% | `core::iter::adapters::try_process::<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, <campfire::controllers::prese` |
| 66.4% | `next<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, campfire::controllers::presenters::{impl#0}::messages::{clos` |
| 66.4% | `try_for_each<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, campfire::control` |
| 66.4% | `try_fold<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models::message::Message>, campfire::controllers::presenters::{impl#0}::messages::{` |
| 66.4% | `try_fold<core::slice::iter::Iter<campfire_db::models::message::Message>, (), core::iter::adapters::map::map_try_fold::{closure_env#0}<&campfire_db::models::mess` |
| 66.4% | `try_fold<core::result::Result<campfire_views::messages::MessageView, campfire_db::error::Error>, core::slice::iter::Iter<campfire_db::models::message::Message>,` |
| 66.3% | `{closure}<&campfire_db::models::message::Message, core::result::Result<campfire_views::messages::MessageView, campfire_db::error::Error>, (), core::ops::control` |
| 66.3% | `<campfire::controllers::presenters::Presenter>::message` |
| 66.0% | `campfire::renderable_message` |
| 63.9% | `spec_extend<campfire_views::messages::MessageView, core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<core::slice::iter::Iter<campfire_db::models` |
| 63.9% | `extend_desugared<campfire_views::messages::MessageView, alloc::alloc::Global, core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<core::slice::ite` |
| 52.3% | `campfire::content` |
| 26.0% | `campfire_richtext::present_message` |
| 26.0% | `campfire_richtext::message_presentation` |
| 26.0% | `<tokio::runtime::context::scoped::Scoped<tokio::runtime::scheduler::Context>>::set::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure},` |
| 26.0% | `with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closur` |
| 26.0% | `<tokio::runtime::task::harness::Harness<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure` |
| 26.0% | `catch_unwind<core::task::poll::Poll<()>, core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::` |
| 26.0% | `tokio::runtime::context::runtime::enter_runtime::<tokio::runtime::scheduler::multi_thread::worker::run::{closure}, ()>` |
| 26.0% | `do_call<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<toki` |
| 26.0% | `<tokio::runtime::task::core::Core<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}>, to` |
| 26.0% | `poll_inner<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::blo` |
| 26.0% | `poll_future<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bl` |
| 26.0% | `{closure}<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 26.0% | `<tokio::runtime::blocking::task::BlockingTask<<tokio::runtime::scheduler::multi_thread::worker::Launch>::launch::{closure}> as core::future::future::Future>::po` |
| 26.0% | `with_mut<tokio::runtime::task::core::Stage<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{clos` |
| 26.0% | `tokio::runtime::scheduler::multi_thread::worker::run` |
| 26.0% | `call_once<core::task::poll::Poll<()>, tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::s` |
| 26.0% | `{closure}<tokio::runtime::blocking::task::BlockingTask<tokio::runtime::scheduler::multi_thread::worker::{impl#0}::launch::{closure_env#0}>, tokio::runtime::bloc` |
| 26.0% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<tokio::runtime::blocking::task::BlockingTask` |
| 26.0% | `try_with<tokio::runtime::context::Context, tokio::runtime::context::set_scheduler::{closure_env#0}<(), tokio::runtime::scheduler::multi_thread::worker::run::{cl` |
| 26.0% | `set_scheduler<(), tokio::runtime::scheduler::multi_thread::worker::run::{closure}::{closure_env#0}>` |
| 26.0% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run` |
| 25.6% | `<tokio::runtime::scheduler::multi_thread::worker::Context>::run_task` |
| 25.5% | `budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler::mult` |
| 25.5% | `with_budget<core::result::Result<alloc::boxed::Box<tokio::runtime::scheduler::multi_thread::worker::Core, alloc::alloc::Global>, ()>, tokio::runtime::scheduler:` |
| 25.4% | `run<alloc::sync::Arc<tokio::runtime::scheduler::multi_thread::handle::Handle, alloc::alloc::Global>>` |
| 25.4% | `<tokio::runtime::task::harness::Harness<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::r` |
| 25.4% | `poll_inner<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runt` |
| 25.4% | `{closure}<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::runti` |
| 25.4% | `<tokio::runtime::task::core::Core<campfire_kit::server::serve<campfire::app::serve::{closure}::{closure}>::{closure}::{closure}, alloc::sync::Arc<tokio::runtime` |
| 25.4% | `catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<tokio::runtime::task::harness::poll_future::{closure_env#0}<campfire_kit::server::serve::{async_fn#0}::{` |
| 25.4% | `poll_future<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0}>, alloc::sync::Arc<tokio::run` |
| 25.4% | `with_mut<tokio::runtime::task::core::Stage<campfire_kit::server::serve::{async_fn#0}::{async_block_env#1}<campfire::app::serve::{async_fn#0}::{async_block_env#0` |
