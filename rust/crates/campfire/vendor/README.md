# Rails icon catalog

`icons.yml` is a byte-identical copy of the reference app's `config/icons.yml`.
The Markdown renderer includes it directly, so image builds need no Rails
icon-catalog file outside the Rust build context.

To refresh it from the repository root (or set `CAMPFIRE_REFERENCE` to another
reference checkout):

```sh
cp "${CAMPFIRE_REFERENCE:-.}/config/icons.yml" rust/crates/campfire/vendor/icons.yml
```

`rich_text::tests::vendored_icon_catalog_matches_reference` compares the raw
bytes against that reference at test time. A missing reference fails under
`CI`; local standalone tests explicitly report the skipped comparison.
