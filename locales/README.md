# UI locale catalogs

Locale files are embedded automatically by `build.rs`. To add a language,
add `locales/<locale-tag>.json` (for example `pt-BR.json`) and rebuild; no
Rust enum, language registry, or preference migration is needed.

```json
{
  "name": "Português (Brasil)",
  "fallback": "en",
  "messages": {
    "menu.edit": "Editar"
  }
}
```

- Use BCP-47 locale tags for filenames (`fr`, `zh-Hans`, `pt-BR`). The
  preference stores this tag as a string.
- `name` is the language's native name shown in the language picker.
- `fallback` names another installed catalog. `ja` is the required root
  catalog; fallback references and cycles are validated during the build.
- Message IDs are stable, language-neutral identifiers. Add the ID to `ja`
  first, then translate it in other catalogs. Missing translations fall
  through the configured fallback chain; the build reports their count.
- Do not use translated sentences as keys or change an ID just to reword a
  label. That keeps catalogs and code changes independent.

Catalog values are plain strings. Rust call sites can use named placeholders
through `i18n::format`; translators may reorder placeholders to fit the
language. Locale-aware plural/date/number formatting is not yet implemented.
