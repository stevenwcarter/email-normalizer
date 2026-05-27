# Extract `email-normalizer` crate from `rc-voting`

**Date:** 2026-05-27
**Status:** Approved for implementation

## Summary

Lift the `email_norm` module out of `~/src/rc-voting` and into a standalone Rust crate at `/home/steve/src/email-normalizer/`. The new crate is a pure, dependency-light email canonicalization library — provider-aware (Gmail dots/plus folding, Yahoo dash subaddresses, iCloud/Outlook alias unification) — published publicly on GitHub but **not** to crates.io. `rc-voting` is **not** modified by this work; it keeps its existing in-tree copy.

## Goals

- A self-contained Rust crate that compiles, tests, lints, and documents cleanly on stable Rust via GitHub Actions on `ubuntu-latest`.
- Behaviorally identical to the `email_norm` module currently in `rc-voting` (same `Email::normalize()` outputs for every test case).
- Smaller dependency surface than the rc-voting copy (no Diesel, no SQLite).
- Ready to be consumed via a git path/dep by any future personal project.

## Non-goals

- Publishing to crates.io. (User may decide to later; not blocking.)
- Any change to `~/src/rc-voting`. The in-tree `email_norm` stays put.
- DB integration. Diesel `ToSql`/`FromSql` impls are dropped; downstream consumers wrap the types themselves if they need DB plumbing.
- New normalization features (additional providers, configurable rules, fuzzy matching, etc.). Future work.

## Source mapping

| rc-voting path | new-crate path | change |
|----------------|----------------|--------|
| `src/email_norm/mod.rs` | `src/lib.rs` | drop the Diesel sub-module and the `#[cfg_attr(feature = "ssr", derive(...))]` derives; drop the `feature = "ssr"` gate on the `mod tests;` declaration |
| `src/email_norm/rules.rs` | `src/rules.rs` | verbatim |
| `src/email_norm/tests.rs` | `src/tests.rs` | drop the `#[cfg(feature = "ssr")]` gate on `normalized_email_serde_is_transparent`; keep the body |

## Public API

Unchanged from the rc-voting module. Two newtypes:

- **`Email`** — raw, user-provided input. Constructors: `From<String>`, `From<&str>`. Accessors: `as_str()`, `AsRef<str>`, `Display`. Method `normalize(&self) -> Option<NormalizedEmail>`. Derives `Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`. `#[serde(transparent)]`.

- **`NormalizedEmail`** — output of `Email::normalize`. **No public constructor** outside the crate (the tuple field is private). Accessors: `as_str()`, `AsRef<str>`, `Display`. Derives `Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`. `#[serde(transparent)]`.

Provider rules (private module): Gmail, iCloud, Outlook, Yahoo, plus a default rule. Rules are baked into `static` data; no runtime configuration.

## Normalization algorithm (preserved verbatim)

1. Trim whitespace.
2. Split on the **last** `@`. Reject if either side is empty.
3. Lowercase both sides.
4. Look up the domain in the alias table → canonical domain (or keep original if unknown).
5. Look up the canonical domain's rule (or `DEFAULT_RULE`).
6. Apply local-part rules in order: strip after `+` if `strip_plus`, strip after `-` if `strip_dash`, remove `.` if `strip_dots`. Reject if the local part is empty after stripping.
7. Return `Some(format!("{local}@{domain}").to_ascii_uppercase())`.

## File layout

```
email-normalizer/
├── Cargo.toml
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── .gitignore
├── rustfmt.toml
├── .github/
│   └── workflows/
│       └── ci.yml
├── docs/
│   └── superpowers/
│       └── specs/
│           └── 2026-05-27-extract-email-normalizer-crate-design.md  (this file)
└── src/
    ├── lib.rs
    ├── rules.rs
    └── tests.rs
```

## `Cargo.toml`

```toml
[package]
name = "email-normalizer"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
description = "Provider-aware email canonicalization (Gmail dots/plus, Yahoo dash, iCloud / Outlook aliases)."
repository = "https://github.com/stevenwcarter/email-normalizer"
readme = "README.md"
keywords = ["email", "normalize", "canonicalize", "deduplication"]
categories = ["email", "text-processing"]

[dependencies]
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
serde_json = "1"
```

No `rust-toolchain.toml`. Stable Rust is sufficient (let-chains stable since 1.88, June 2025; edition 2024 stable since 1.85, Feb 2025).

## LICENSE files

Two files at the repo root:

- **`LICENSE-MIT`** — standard MIT text, `Copyright (c) 2026 Steve Carter`.
- **`LICENSE-APACHE`** — standard Apache 2.0 text (verbatim from https://www.apache.org/licenses/LICENSE-2.0.txt).

The `Cargo.toml` `license = "MIT OR Apache-2.0"` SPDX expression is what consumers and tooling read; the two files are the canonical full texts for the repo.

## `README.md`

Short. Sections:

1. **One-paragraph what/why** — explains the crate canonicalizes email addresses per provider quirks so consumers can use the output for uniqueness/lookup keys.
2. **Install** — `[dependencies] email-normalizer = { git = "https://github.com/stevenwcarter/email-normalizer" }` (since not on crates.io).
3. **Usage** — a single example showing `Email::from("Steve+work@Gmail.com").normalize().unwrap()` → `"STEVE@GMAIL.COM"`.
4. **Provider rules** — short table: Gmail (strip `+`, strip `.`, alias `googlemail.com`), iCloud (strip `+`, aliases `me.com`/`mac.com`), Outlook (strip `+`, aliases `hotmail.*`/`live.*`/`msn.com` + regional TLDs), Yahoo (strip `-`, aliases `yahoo.*` regional TLDs + `ymail.com`/`rocketmail.com`), default (strip `+` only, keep domain).
5. **Caveats** — output is uppercase canonical form, **not** a valid email for sending; never serialize it to users; provider rules are heuristics and providers may change behavior.
6. **License** — MIT OR Apache-2.0 footer.

## `.gitignore`

Minimal Rust .gitignore:

```
/target
```

`Cargo.lock` **is** committed, per current Cargo guidance — reproducible CI for the library itself; consumers ignore it transitively.

## `rustfmt.toml`

Matches rc-voting's:

```toml
edition = "2024"
```

## CI workflow (`.github/workflows/ci.yml`)

Single job, runs on `ubuntu-latest`, triggers on push to `main` and on pull requests:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: cargo fmt --check
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: cargo test
        run: cargo test
      - name: cargo doc
        env:
          RUSTDOCFLAGS: -D warnings
        run: cargo doc --no-deps
```

Self-hosted runners and Docker images are explicitly out of scope — `ubuntu-latest` is correct for a public personal project.

## Testing strategy

The test suite is the existing 30+ unit tests in `tests.rs`, ported verbatim minus the `feature = "ssr"` gate. They cover:

- Basic normalization, whitespace trim, case folding
- Rejections (no `@`, empty local, empty domain, all-whitespace, empty string, local empty after plus-strip)
- Last-`@` split behavior for malformed multi-`@` inputs
- Per-provider rule application (Gmail, Yahoo, iCloud, Outlook, default)
- Alias resolution (googlemail → gmail, ymail/rocketmail → yahoo, me.com/mac.com → icloud, hotmail/live/msn → outlook, regional TLDs)
- Idempotency (normalize ∘ normalize == normalize) across all rule families
- Serde transparency (`NormalizedEmail` JSON-serializes to a bare string)

`cargo test` is the only test invocation. No integration tests, no doctests beyond what the README example provides (which is a code block, not a doctest, since the README isn't included via `#![doc = include_str!(...)]`).

## Behavioral parity check (acceptance)

Beyond `cargo test` passing, after extraction:

1. The crate's `src/lib.rs`, `src/rules.rs`, `src/tests.rs` differ from rc-voting's `src/email_norm/{mod.rs,rules.rs,tests.rs}` **only** in:
   - The `mod tests;` declaration (no `feature = "ssr"` gate).
   - The removed Diesel `cfg_attr` derives on both newtypes.
   - The removed `#[cfg(feature = "ssr")] mod diesel_impls { ... }` block.
   - The removed `#[cfg(feature = "ssr")]` gate on `normalized_email_serde_is_transparent`.
   - The `mod rules;` path (still `mod rules;` since `rules.rs` is now a sibling of `lib.rs`).

   A `diff -u` between corresponding files should show exactly these changes and nothing else.

2. The 30+ tests pass.

3. The crate builds clean under `cargo doc` with `-D warnings`.

## Implementation order

1. Initialize the crate skeleton: `Cargo.toml`, `.gitignore`, `rustfmt.toml`, both LICENSE files, README.md, the CI workflow file.
2. Port `src/lib.rs` from `rc-voting/src/email_norm/mod.rs` with the listed deletions.
3. Port `src/rules.rs` verbatim from `rc-voting/src/email_norm/rules.rs`.
4. Port `src/tests.rs` from `rc-voting/src/email_norm/tests.rs` with the listed deletions.
5. Run locally: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo doc --no-deps`.
6. Initial commit. Push to `https://github.com/stevenwcarter/email-normalizer` when the user is ready (out of scope for this work — local repo only).
