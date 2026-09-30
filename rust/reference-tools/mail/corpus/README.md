These 32 HTML structures come from actual email in the Apache SpamAssassin public corpus:

- [20030228_hard_ham.tar.bz2](https://spamassassin.apache.org/old/publiccorpus/20030228_hard_ham.tar.bz2)
- [20030228_spam.tar.bz2](https://spamassassin.apache.org/old/publiccorpus/20030228_spam.tar.bz2)
- [Corpus provenance and terms](https://spamassassin.apache.org/old/publiccorpus/readme.html)

Each case records its archive SHA256, original member name and MIME part number. The importer
selects the first 16 HTML messages in each archive, ordered by member name, whose decoded HTML
is 500–40,000 bytes. It preserves tag names, nesting, entities and whitespace, while replacing
letters/digits in prose and attribute values with `x` and replacing declarations/comments.
Original message prose, identifiers, URLs and addresses are not retained. This transformation
keeps the malformed HTML structures that matter to text extraction; it does not preserve the
original visible content. These are an older corpus, not coverage of all modern mail clients.

Downloads stay in the worktree's ignored target directory. Regenerate the sanitized input:

```sh
python3 rust/reference-tools/mail/import-html-corpus.py \
  rust/target/ws10-corpus/hard-ham.tar.bz2 rust/target/ws10-corpus/spam.tar.bz2
```

Then `generate.sh` runs the sanitized cases through our own `RoomMailbox#html_to_text` and
writes their expected outputs alongside the generated edge cases. Tests only parse strings;
they never deliver corpus mail or fetch embedded resources.

`public-auth-headers.json` contains two sanitized structures from emails their original
reporters posted in [a Python email bug example](https://gist.github.com/pazz/953e0f7cce8220c59d742aecc8779bb7)
and [an EML parsing bug report](https://github.com/Unstructured-IO/unstructured/issues/1546).
Relay names, domains, identifiers and comment prose are replaced with fixture values. The
headers preserve their original folding, method/property selection and clause ordering.
Their provenance is recorded per case; expected trust results are computed by our Ruby.
