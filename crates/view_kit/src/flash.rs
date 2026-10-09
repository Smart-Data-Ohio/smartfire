//! Track actual template access to Rails' lazy flash hash on the rendering thread.
use std::cell::Cell;
thread_local! {
    static READS: Cell<u64> = const { Cell::new(0) };
}
/// A template touched `flash[:notice]` or `flash[:alert]`. Retained pages call this too.
pub fn read() {
    READS.with(|reads| reads.set(reads.get().wrapping_add(1)));
}
/// Nested renders contribute to the enclosing render; no flash/session data is retained.
pub fn track_reads<R>(render: impl FnOnce() -> R) -> (R, bool) {
    let before = READS.with(Cell::get);
    let result = render();
    (result, READS.with(Cell::get) != before)
}
