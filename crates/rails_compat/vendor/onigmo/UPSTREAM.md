# Pinned Ruby regex engine

Source: https://github.com/ruby/ruby/tree/2b0b7728dc7f0561c35c3d8c4489945c94b783ad
Ruby 3.4.10, the exact RUBY_REVISION in triage-reference-d7c7de92.
Unicode tables: 15.0.0. Copyright and redistribution terms remain in each file,
COPYING and BSDL. st.c / st.h are public domain.

All engine C files and Unicode tables are byte-identical to that revision.
UPSTREAM.json records original paths and hashes. st.h has one mechanical adaptation:
rb_st_ is renamed onig_st_ to match the standalone engine's symbol namespace.
config.h supplies standalone platform/compiler macros; ruby/defines.h supplies only
hash-table declaration macros/types, and internal/sanitizers.h omits Ruby allocator
instrumentation. No regex matching, folding or search optimization is modified.
Compile with NOT_RUBY. No Ruby allocator, VM, shared library or runtime is linked.

The owned Rust bridge lives in crates/rails_compat/native/keyword_regex.c. It exposes
only compilation/search/destruction. Rust builds escaped literal keyword patterns
with the reference's whitespace and Unicode boundary pattern. A process mutex covers
engine initialization, compilation, matching and freeing, including its shared caches.
