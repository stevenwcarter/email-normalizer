# Extract `email-normalizer` Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the `email_norm` module from `~/src/rc-voting` into a standalone Rust crate at `/home/steve/src/email-normalizer/` — a pure, dependency-light email canonicalization library published publicly on GitHub (not crates.io).

**Architecture:** Single library crate with two public newtypes (`Email`, `NormalizedEmail`), one private `rules` submodule, one private `tests` submodule. `serde` is the only runtime dependency. No Diesel, no DB integration. CI on `ubuntu-latest`.

**Tech Stack:** Rust 2024 edition, stable toolchain (no MSRV pin), `serde` 1.x, GitHub Actions.

**Source of truth for code:** `/home/steve/src/rc-voting/src/email_norm/{mod.rs,rules.rs,tests.rs}`. The new crate's `src/{lib.rs,rules.rs,tests.rs}` are direct ports with the deletions enumerated in the spec.

**Working directory:** `/home/steve/src/email-normalizer/` (already `git init`'d on `main`, contains only `docs/`).

---

## Task 1: Crate skeleton — `Cargo.toml`, `.gitignore`, `rustfmt.toml`

**Files:**
- Create: `/home/steve/src/email-normalizer/Cargo.toml`
- Create: `/home/steve/src/email-normalizer/.gitignore`
- Create: `/home/steve/src/email-normalizer/rustfmt.toml`

- [ ] **Step 1: Create `Cargo.toml`**

Write to `/home/steve/src/email-normalizer/Cargo.toml`:

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

- [ ] **Step 2: Create `.gitignore`**

Write to `/home/steve/src/email-normalizer/.gitignore`:

```
/target
```

(Cargo.lock IS committed — current Cargo guidance for libraries with CI.)

- [ ] **Step 3: Create `rustfmt.toml`**

Write to `/home/steve/src/email-normalizer/rustfmt.toml`:

```toml
edition = "2024"
```

- [ ] **Step 4: Commit the skeleton**

```bash
cd /home/steve/src/email-normalizer
git add Cargo.toml .gitignore rustfmt.toml
git commit -m "Add Cargo manifest and toolchain config"
```

---

## Task 2: License files

**Files:**
- Create: `/home/steve/src/email-normalizer/LICENSE-MIT`
- Create: `/home/steve/src/email-normalizer/LICENSE-APACHE`

- [ ] **Step 1: Create `LICENSE-MIT`**

Write to `/home/steve/src/email-normalizer/LICENSE-MIT`:

```
MIT License

Copyright (c) 2026 Steven Carter

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Fetch and write `LICENSE-APACHE`**

Run:

```bash
curl -sSL https://www.apache.org/licenses/LICENSE-2.0.txt \
  -o /home/steve/src/email-normalizer/LICENSE-APACHE
```

Verify the file is non-empty and starts with "Apache License":

```bash
head -2 /home/steve/src/email-normalizer/LICENSE-APACHE
```

Expected first line: `                                 Apache License`

If `curl` is unavailable or the fetch fails, write the full Apache 2.0 license text directly to the file (text is freely redistributable; copy from any open-source Rust crate that uses Apache 2.0, e.g. `~/src/rc-voting`'s ecosystem deps).

- [ ] **Step 3: Commit license files**

```bash
cd /home/steve/src/email-normalizer
git add LICENSE-MIT LICENSE-APACHE
git commit -m "Add MIT and Apache-2.0 license files"
```

---

## Task 3: Port `src/rules.rs` verbatim

**Files:**
- Create: `/home/steve/src/email-normalizer/src/rules.rs`
- Source: `/home/steve/src/rc-voting/src/email_norm/rules.rs`

- [ ] **Step 1: Create `src/` directory**

```bash
mkdir -p /home/steve/src/email-normalizer/src
```

- [ ] **Step 2: Copy `rules.rs` verbatim**

```bash
cp /home/steve/src/rc-voting/src/email_norm/rules.rs \
   /home/steve/src/email-normalizer/src/rules.rs
```

- [ ] **Step 3: Verify the copy matches the source exactly**

```bash
diff /home/steve/src/rc-voting/src/email_norm/rules.rs \
     /home/steve/src/email-normalizer/src/rules.rs
```

Expected: no output (files are identical).

- [ ] **Step 4: Do NOT commit yet** — lib.rs/tests.rs are needed first to make the crate compile. Move on to Task 4.

---

## Task 4: Port `src/lib.rs` (was `mod.rs`, with Diesel removed)

**Files:**
- Create: `/home/steve/src/email-normalizer/src/lib.rs`
- Source: `/home/steve/src/rc-voting/src/email_norm/mod.rs`

- [ ] **Step 1: Write `src/lib.rs`**

Write the following EXACTLY to `/home/steve/src/email-normalizer/src/lib.rs` (this is the rc-voting `mod.rs` with all four Diesel-related items removed and the test-gate simplified):

```rust
//! Provider-aware email normalization. Returns an uppercase canonical
//! form suitable for unique-constraint enforcement and lookup.
//! Never serialized to the frontend.

mod rules;

#[cfg(test)]
mod tests;

fn normalize_str(email: &str) -> Option<String> {
    let trimmed = email.trim();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }

    let local = local.to_ascii_lowercase();
    let domain = domain.to_ascii_lowercase();

    // Resolve domain via alias table.
    let domain = rules::resolve_domain(&domain).to_string();

    // Pick the rule for the resolved domain (or DEFAULT_RULE).
    let rule = rules::rule_for_domain(&domain);

    // Apply local-part rules in order: + → - → dots.
    let mut local = local;
    if rule.strip_plus
        && let Some(idx) = local.find('+')
    {
        local.truncate(idx);
    }
    if rule.strip_dash
        && let Some(idx) = local.find('-')
    {
        local.truncate(idx);
    }
    if rule.strip_dots {
        local = local.replace('.', "");
    }

    if local.is_empty() {
        return None;
    }

    Some(format!("{local}@{domain}").to_ascii_uppercase())
}

/// Raw, user-provided email. No validation; typed wrapper around String.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The only public constructor for `NormalizedEmail`.
    pub fn normalize(&self) -> Option<NormalizedEmail> {
        normalize_str(&self.0).map(NormalizedEmail)
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Email {
    fn from(s: String) -> Self {
        Email(s)
    }
}

impl From<&str> for Email {
    fn from(s: &str) -> Self {
        Email(s.to_string())
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Normalized email — output of `Email::normalize`. Cannot be constructed
/// from outside this module.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct NormalizedEmail(String);

impl NormalizedEmail {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NormalizedEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for NormalizedEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

**Diff vs `rc-voting/src/email_norm/mod.rs`:** removed `#[cfg(all(test, feature = "ssr"))]` (now `#[cfg(test)]`); removed both `#[cfg_attr(feature = "ssr", derive(diesel::AsExpression, diesel::FromSqlRow))]` and `#[cfg_attr(feature = "ssr", diesel(sql_type = diesel::sql_types::Text))]` from both `Email` and `NormalizedEmail`; removed the entire trailing `#[cfg(feature = "ssr")] mod diesel_impls { ... }` block.

- [ ] **Step 2: Do NOT commit yet** — tests.rs is still missing; the crate won't compile (the `mod tests;` declaration points to a missing file). Proceed to Task 5.

---

## Task 5: Port `src/tests.rs`

**Files:**
- Create: `/home/steve/src/email-normalizer/src/tests.rs`
- Source: `/home/steve/src/rc-voting/src/email_norm/tests.rs`

- [ ] **Step 1: Copy `tests.rs`**

```bash
cp /home/steve/src/rc-voting/src/email_norm/tests.rs \
   /home/steve/src/email-normalizer/src/tests.rs
```

- [ ] **Step 2: Remove the `#[cfg(feature = "ssr")]` gate on the serde test**

The original file has, near the bottom:

```rust
#[cfg(feature = "ssr")]
#[test]
fn normalized_email_serde_is_transparent() {
```

In the new crate, serde is always-on, so the gate must go.

Edit `/home/steve/src/email-normalizer/src/tests.rs`:
- Find: `#[cfg(feature = "ssr")]\n#[test]\nfn normalized_email_serde_is_transparent()`
- Replace with: `#[test]\nfn normalized_email_serde_is_transparent()`

(Concretely: delete the `#[cfg(feature = "ssr")]` line immediately preceding `fn normalized_email_serde_is_transparent`. Leave the `#[test]` attribute and the function body untouched.)

- [ ] **Step 3: Verify the diff vs source is exactly the one removed line**

```bash
diff /home/steve/src/rc-voting/src/email_norm/tests.rs \
     /home/steve/src/email-normalizer/src/tests.rs
```

Expected output: a single hunk showing the removed `#[cfg(feature = "ssr")]` line and nothing else.

---

## Task 6: First end-to-end verification — fmt, clippy, test, doc

**Files:** none (verification only).

- [ ] **Step 1: Format check**

```bash
cd /home/steve/src/email-normalizer
cargo fmt --all -- --check
```

Expected: no output, exit 0. If formatter complains, run `cargo fmt --all` to fix, re-run `--check`, then move on.

- [ ] **Step 2: Clippy with warnings as errors**

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: clean compilation, exit 0. If clippy fires, fix the specific lint in-place (these are ports of code that already passed clippy in rc-voting under the same `-D warnings` discipline, so anything new is a real signal).

- [ ] **Step 3: Run the test suite**

```bash
cargo test
```

Expected: All 30+ tests pass. Specifically you should see tests like `normalizes_basic_address_to_uppercase`, `gmail_strips_plus_subaddress`, `yahoo_regional_tld_maps_to_yahoo_com`, `idempotency_holds_for_each_provider`, `normalized_email_serde_is_transparent` etc. all reported as `ok`.

If a test fails, the most likely cause is a botched port — re-read the corresponding section of the rc-voting source and reconcile.

- [ ] **Step 4: Doc build with warnings as errors**

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Expected: clean build, exit 0.

- [ ] **Step 5: Commit the working library**

```bash
cd /home/steve/src/email-normalizer
git add src/ Cargo.lock
git commit -m "Port email-normalizer library and tests from rc-voting"
```

---

## Task 7: `README.md`

**Files:**
- Create: `/home/steve/src/email-normalizer/README.md`

- [ ] **Step 1: Write `README.md`**

Write to `/home/steve/src/email-normalizer/README.md`:

````markdown
# email-normalizer

Provider-aware canonicalization of email addresses. Folds away the noise that
mail providers themselves treat as equivalent — Gmail dot-and-plus
subaddressing, Yahoo dash subaddresses, iCloud and Outlook alias domains — so
the output is a stable key suitable for unique-constraint enforcement,
deduplication, and lookup.

The canonical form is uppercase and **not** a deliverable address. Never display
it to users; never use it as a `mailto:` target.

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

## Rules

| Provider | `+`  | `-`  | `.`  | Aliases folded into canonical domain |
|----------|------|------|------|--------------------------------------|
| Gmail    | strip | keep | strip | `googlemail.com` → `gmail.com` |
| iCloud   | strip | keep | keep | `me.com`, `mac.com` → `icloud.com` |
| Outlook  | strip | keep | keep | `hotmail.*`, `live.*`, `msn.com`, regional `outlook.*` → `outlook.com` |
| Yahoo    | keep  | strip | keep | regional `yahoo.*`, `ymail.com`, `rocketmail.com` → `yahoo.com` |
| Default  | strip | keep | keep | domain unchanged |

Domain matching is case-insensitive. The output is always uppercase.

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
````

- [ ] **Step 2: Commit the README**

```bash
cd /home/steve/src/email-normalizer
git add README.md
git commit -m "Add README"
```

---

## Task 8: GitHub Actions CI workflow

**Files:**
- Create: `/home/steve/src/email-normalizer/.github/workflows/ci.yml`

- [ ] **Step 1: Create workflows directory**

```bash
mkdir -p /home/steve/src/email-normalizer/.github/workflows
```

- [ ] **Step 2: Write `ci.yml`**

Write to `/home/steve/src/email-normalizer/.github/workflows/ci.yml`:

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

- [ ] **Step 3: Commit the workflow**

```bash
cd /home/steve/src/email-normalizer
git add .github/workflows/ci.yml
git commit -m "Add CI workflow (fmt, clippy, test, doc) on ubuntu-latest"
```

---

## Task 9: Final verification + parity check vs rc-voting

**Files:** none (verification only).

- [ ] **Step 1: Final clean run of the full CI command set**

```bash
cd /home/steve/src/email-normalizer
cargo fmt --all -- --check && \
cargo clippy --all-targets -- -D warnings && \
cargo test && \
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Expected: all four steps pass with exit 0.

- [ ] **Step 2: Parity diff against rc-voting source**

Confirm the only differences between the ported files and the rc-voting originals are the ones the spec enumerates.

```bash
# rules.rs should be byte-identical
diff /home/steve/src/rc-voting/src/email_norm/rules.rs \
     /home/steve/src/email-normalizer/src/rules.rs
```

Expected: no output.

```bash
# tests.rs should differ ONLY by the removed `#[cfg(feature = "ssr")]` line
diff /home/steve/src/rc-voting/src/email_norm/tests.rs \
     /home/steve/src/email-normalizer/src/tests.rs
```

Expected: one hunk removing `#[cfg(feature = "ssr")]` (no other changes).

```bash
# lib.rs vs mod.rs: should differ ONLY by the items listed in the spec:
#   - `#[cfg(all(test, feature = "ssr"))]` → `#[cfg(test)]`
#   - two removed `#[cfg_attr(feature = "ssr", derive(diesel::...))]` lines
#   - two removed `#[cfg_attr(feature = "ssr", diesel(sql_type = ...))]` lines
#   - removed `#[cfg(feature = "ssr")] mod diesel_impls { ... }` block
diff /home/steve/src/rc-voting/src/email_norm/mod.rs \
     /home/steve/src/email-normalizer/src/lib.rs
```

Expected: hunks matching exactly that list.

- [ ] **Step 3: Inspect git log**

```bash
cd /home/steve/src/email-normalizer
git log --oneline
```

Expected ordering (most recent first):

```
Add CI workflow (fmt, clippy, test, doc) on ubuntu-latest
Add README
Port email-normalizer library and tests from rc-voting
Add MIT and Apache-2.0 license files
Add Cargo manifest and toolchain config
Add design spec: extract email-normalizer crate from rc-voting
```

(Order of intermediate commits may differ slightly; the design spec must be the root commit.)

- [ ] **Step 4: Confirm no rc-voting changes**

```bash
cd /home/steve/src/rc-voting
git status
```

Expected: whatever pre-existing state `rc-voting` is in — this work must NOT have touched any file under `~/src/rc-voting/`.

---

## Done

When all tasks above are checked off, the crate is ready to be pushed to
`https://github.com/stevenwcarter/email-normalizer`. Pushing the remote is
out of scope for this plan — the user will create the GitHub repo and add the
remote manually.
