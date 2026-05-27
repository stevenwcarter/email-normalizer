# email-normalizer

Provider-aware canonicalization of email addresses. Folds away the noise that
mail providers themselves treat as equivalent — Gmail dot-and-plus
subaddressing, Yahoo dash subaddresses, iCloud and Outlook alias domains — so
the output is a stable key suitable for unique-constraint enforcement,
deduplication, and lookup.

The canonical form is uppercase by default (configurable — see below) and
**not** a deliverable address. Never display it to users; never use it as a
`mailto:` target.

## Install

This crate is not on crates.io. Depend on it via git:

```toml
[dependencies]
email-normalizer = { git = "https://github.com/stevenwcarter/email-normalizer" }
```

## Usage

```rust
use email_normalizer::Email;

let raw = Email::from("Steve+work@Gmail.com");
let normalized = raw.normalize().expect("valid email");
assert_eq!(normalized.as_str(), "STEVE@GMAIL.COM");
```

`Email::normalize` returns `None` for inputs with no `@`, empty local or
domain parts, or where stripping a subaddress leaves an empty local part.

### Configuring behavior

For non-default behavior, build a `NormalizerConfig` and call
`Email::normalize_with`:

```rust
use email_normalizer::{Email, NormalizerConfig, OutputCase, ProviderRule};

// Lowercase output instead of the default uppercase.
let cfg = NormalizerConfig::builder()
    .output_case(OutputCase::Lowercase)
    .build();
let n = Email::from("Steve+work@Gmail.com").normalize_with(&cfg).unwrap();
assert_eq!(n.as_str(), "steve@gmail.com");
```

Other toggles on `NormalizerConfig::builder()`:

- `apply_provider_rules(false)` — disable subaddress stripping (gmail dot/plus,
  yahoo dash) and the default plus-strip fallback.
- `resolve_domain_aliases(false)` — keep alias domains like `googlemail.com`
  instead of rewriting to `gmail.com`.
- `add_rule(my_rule)` — plug in your own rule (implements the public
  `ProviderRule` trait). Custom rules are checked before built-ins.
- `use_built_in_rules(false)` — combined with `add_rule`, replaces the
  built-in rules entirely.

## Rules

| Provider | `+`  | `-`  | `.`  | Aliases folded into canonical domain |
|----------|------|------|------|--------------------------------------|
| Gmail    | strip | keep | strip | `googlemail.com` → `gmail.com` |
| iCloud   | strip | keep | keep | `me.com`, `mac.com` → `icloud.com` |
| Outlook  | strip | keep | keep | `hotmail.*`, `live.*`, `msn.com`, regional `outlook.*` → `outlook.com` |
| Yahoo    | keep  | strip | keep | regional `yahoo.*`, `ymail.com`, `rocketmail.com` → `yahoo.com` |
| Default  | strip | keep | keep | domain unchanged |

Domain matching is case-insensitive. The output is uppercase by default;
set `OutputCase::Lowercase` on the config to override.

## Caveats

- Provider rules are heuristics; mail providers may change behavior. Canonical
  forms produced by one version of this crate may not match forms produced by
  a later version. Re-normalize when bumping the crate.
- The crate does not validate that an address is RFC-compliant or deliverable.
  Garbage in, garbage out — `Email::normalize` only rejects structurally empty
  input.
- For multi-`@` malformed inputs the crate splits on the last `@`. Don't rely
  on this for quoted local-parts; this crate doesn't claim to handle them.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
