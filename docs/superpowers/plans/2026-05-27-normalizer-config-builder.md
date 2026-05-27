# NormalizerConfig + ProviderRule trait + builder — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a public `ProviderRule` trait, a `NormalizerConfig` value type with a `NormalizerConfigBuilder`, and an `Email::normalize_with(&NormalizerConfig)` method. Defaults preserve current behavior bit-for-bit; consumers can override case, disable provider rules / alias resolution, stack custom rules on top of built-ins, or replace built-ins entirely.

**Architecture:** Four tasks, each ending in a green test run and a clean commit. Task 1 refactors `rules.rs` to introduce the public trait and a `find_matching_rule` helper without changing behavior. Task 2 adds the config + builder in a new `src/config.rs` (data + construction only; no behavior wired yet). Task 3 wires the config into `normalize_str` and `Email::normalize_with` for the three orthogonal toggles (case, rules, aliases). Task 4 wires custom rules through `find_matching_rule` for stack and replace modes.

**Tech Stack:** Rust 2024 edition, `std::sync::Arc`, no new external crates.

**Spec:** [`docs/superpowers/specs/2026-05-27-normalizer-config-builder-design.md`](../specs/2026-05-27-normalizer-config-builder-design.md)

---

## File map

| File | Responsibility | Task |
|---|---|---|
| `src/rules.rs` | Public `ProviderRule` trait, private `BuiltInRule` struct with trait impl, `BUILT_IN_RULES` slice, `DEFAULT_RULE`, `find_matching_rule` helper | 1 |
| `src/config.rs` (new) | `OutputCase` enum, `NormalizerConfig` struct, `NormalizerConfigBuilder` struct, all builder methods | 2 |
| `src/lib.rs` | `Email`, `NormalizedEmail`, `normalize_str(&str, &NormalizerConfig)`, `Email::normalize_with`, re-exports of public types | 1 (re-export), 3 (full rewrite of `normalize_str`) |
| `src/tests.rs` | All 34 existing tests stay; new tests added per task | 1 (no new tests; regression check), 2, 3, 4 |

---

## Task 1: Add public `ProviderRule` trait, refactor `rules.rs` to expose it; preserve all existing behavior

**Files:**
- Modify: `src/rules.rs` (full rewrite of types; same data)
- Modify: `src/lib.rs:5-47` (update `normalize_str` to call the new `find_matching_rule` helper; add `pub use rules::ProviderRule;` re-export)

**Goal of this task:** introduce the public trait and the helper that future tasks will lean on, without changing any test's output. All 34 existing tests must still pass.

- [ ] **Step 1: Confirm the starting state**

Run:
```bash
cargo test 2>&1 | tail -3
git log --oneline -5
```

Expected: `34 passed`. Top of log shows the spec commit. If `cargo test` reports anything other than 34/0/0, stop and report — the plan assumes a green baseline.

- [ ] **Step 2: Replace `src/rules.rs` with the new content**

Overwrite the file with exactly this content:

```rust
//! Per-provider normalization rules.
//!
//! The public `ProviderRule` trait is what users implement to plug in
//! their own rules. The built-in rules for Gmail, iCloud, Outlook, and
//! Yahoo are private `BuiltInRule` values stored in `BUILT_IN_RULES`;
//! they implement `ProviderRule` like any other rule.
//!
//! `DEFAULT_RULE` is the catch-all applied when no rule matched a given
//! domain. It strips a plus-subaddress and leaves dashes / dots alone.

use std::sync::Arc;

/// A normalization rule for a particular set of domains.
///
/// `Send + Sync` so rules can be shared across threads through
/// `Arc<dyn ProviderRule>`. The trait is object-safe.
pub trait ProviderRule: Send + Sync {
    /// True if this rule applies to `domain` (already lowercased).
    /// A rule matches both its canonical domain and any aliases it owns.
    fn matches_domain(&self, domain: &str) -> bool;

    /// The canonical form of the domain — what `domain` rewrites to when
    /// `matches_domain` returned true.
    fn canonical_domain(&self) -> &str;

    /// Transform the local part. Return value of `""` causes the caller
    /// to reject the whole email (the caller checks for emptiness).
    fn transform_local(&self, local: &str) -> String;
}

struct BuiltInRule {
    canonical_domain: &'static str,
    aliases: &'static [&'static str],
    strip_plus: bool,
    strip_dash: bool,
    strip_dots: bool,
}

impl ProviderRule for BuiltInRule {
    fn matches_domain(&self, domain: &str) -> bool {
        domain == self.canonical_domain || self.aliases.contains(&domain)
    }

    fn canonical_domain(&self) -> &str {
        self.canonical_domain
    }

    fn transform_local(&self, local: &str) -> String {
        let mut s = local.to_string();
        if self.strip_plus
            && let Some(idx) = s.find('+')
        {
            s.truncate(idx);
        }
        if self.strip_dash
            && let Some(idx) = s.find('-')
        {
            s.truncate(idx);
        }
        if self.strip_dots {
            s = s.replace('.', "");
        }
        s
    }
}

static BUILT_IN_RULES: &[BuiltInRule] = &[
    // Gmail / Google Workspace
    BuiltInRule {
        canonical_domain: "gmail.com",
        aliases: &["googlemail.com"],
        strip_plus: true,
        strip_dash: false,
        strip_dots: true,
    },
    // Apple iCloud
    BuiltInRule {
        canonical_domain: "icloud.com",
        aliases: &["me.com", "mac.com"],
        strip_plus: true,
        strip_dash: false,
        strip_dots: false,
    },
    // Microsoft (Outlook / Hotmail / Live / MSN, with common regional TLDs)
    BuiltInRule {
        canonical_domain: "outlook.com",
        aliases: &[
            "hotmail.com",
            "hotmail.co.uk",
            "hotmail.fr",
            "hotmail.de",
            "hotmail.it",
            "hotmail.es",
            "live.com",
            "live.co.uk",
            "live.fr",
            "live.de",
            "msn.com",
            "outlook.co.uk",
            "outlook.fr",
            "outlook.de",
            "outlook.com.au",
        ],
        strip_plus: true,
        strip_dash: false,
        strip_dots: false,
    },
    // Yahoo (uses `-` as subaddress separator)
    BuiltInRule {
        canonical_domain: "yahoo.com",
        aliases: &[
            "yahoo.co.uk",
            "yahoo.co.jp",
            "yahoo.co.in",
            "yahoo.fr",
            "yahoo.de",
            "yahoo.it",
            "yahoo.es",
            "yahoo.ie",
            "yahoo.ca",
            "yahoo.com.au",
            "yahoo.com.br",
            "yahoo.com.mx",
            "yahoo.com.ar",
            "yahoo.com.sg",
            "yahoo.com.ph",
            "yahoo.com.hk",
            "yahoo.com.tw",
            "yahoo.com.vn",
            "ymail.com",
            "rocketmail.com",
        ],
        strip_plus: false,
        strip_dash: true,
        strip_dots: false,
    },
];

pub(crate) static DEFAULT_RULE: BuiltInRule = BuiltInRule {
    canonical_domain: "",
    aliases: &[],
    strip_plus: true,
    strip_dash: false,
    strip_dots: false,
};

/// Find the first rule that matches `domain`. Custom rules are checked
/// before built-ins. Built-ins are skipped when `use_built_in_rules` is
/// false.
pub(crate) fn find_matching_rule<'a>(
    domain: &str,
    custom_rules: &'a [Arc<dyn ProviderRule>],
    use_built_in_rules: bool,
) -> Option<&'a dyn ProviderRule> {
    for rule in custom_rules {
        if rule.matches_domain(domain) {
            return Some(rule.as_ref());
        }
    }
    if use_built_in_rules {
        for rule in BUILT_IN_RULES {
            if rule.matches_domain(domain) {
                return Some(rule);
            }
        }
    }
    None
}
```

Diff summary vs. existing `rules.rs`:
- `pub struct ProviderRule` → `struct BuiltInRule` (renamed, made private).
- `PROVIDER_RULES: &[ProviderRule]` → `static BUILT_IN_RULES: &[BuiltInRule]` (renamed, made private, `static` instead of `const` because `BuiltInRule` now has trait impl methods — `const` arrays of types with trait impls are awkward).
- `DEFAULT_RULE: ProviderRule` → `pub(crate) static DEFAULT_RULE: BuiltInRule` (made `pub(crate)` so `lib.rs` can reference it; `static` to match the array).
- New `pub trait ProviderRule` with three methods. Object-safe.
- `impl ProviderRule for BuiltInRule { ... }` wires the three trait methods to the existing flag-driven local-part pipeline.
- New `pub(crate) fn find_matching_rule(...)` replaces both `resolve_domain` and `rule_for_domain`. Returns `Option<&dyn ProviderRule>` — the matched rule, or None if nothing matched.
- `resolve_domain` and `rule_for_domain` are removed (their callers in lib.rs change in Step 3).

- [ ] **Step 3: Update `src/lib.rs` to use the new helper**

Edit `src/lib.rs`. Replace lines 5-47 (the `mod rules;` line through the end of `normalize_str`) with:

```rust
mod config;
mod rules;

#[cfg(test)]
mod tests;

pub use rules::ProviderRule;

fn normalize_str(email: &str) -> Option<String> {
    let trimmed = email.trim();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }

    let mut local = local.to_ascii_lowercase();
    let mut domain = domain.to_ascii_lowercase();

    let matched = rules::find_matching_rule(&domain, &[], true);

    if let Some(rule) = matched {
        domain = rule.canonical_domain().to_string();
        local = rule.transform_local(&local);
    } else {
        local = rules::DEFAULT_RULE.transform_local(&local);
    }

    if local.is_empty() {
        return None;
    }

    Some(format!("{local}@{domain}").to_ascii_uppercase())
}
```

Notes:
- `mod config;` is declared here so Task 2 can drop in the file and have it pick up immediately. It will fail compilation until `config.rs` exists, which is why we **create an empty config.rs in the next step**.
- The signature of `normalize_str` is unchanged — still `(email: &str) -> Option<String>`. Task 3 changes this signature.
- `find_matching_rule(&domain, &[], true)` — empty custom_rules slice, built-ins on. Preserves current behavior exactly.
- Existing rest of file (`Email`, `NormalizedEmail`, and all impls below the changed region) stays untouched.

- [ ] **Step 4: Create an empty `src/config.rs` so the crate compiles**

Create the file with this single-line content:

```rust
// Filled in by Task 2.
```

This satisfies the `mod config;` declaration in `lib.rs`. Without it, `cargo check` fails at "file not found for module `config`".

- [ ] **Step 5: Run tests; expect all 34 to pass**

Run: `cargo test 2>&1 | tail -10`

Expected: includes `test result: ok. 34 passed; 0 failed`. If any test fails, the refactor changed behavior somewhere — debug before proceeding.

- [ ] **Step 6: Run clippy; expect clean**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`

Expected: no warnings. (Clippy may complain about `static` items with interior mutability or about `pub(crate)` visibility on items that don't need it — investigate any warning, don't `#[allow]` it without understanding.)

- [ ] **Step 7: Commit**

```bash
git add src/rules.rs src/lib.rs src/config.rs
git commit -m "$(cat <<'EOF'
Introduce public ProviderRule trait; refactor rules.rs internals

Public trait surface so consumers can implement their own rules.
Built-in rules (Gmail/iCloud/Outlook/Yahoo) are now private
BuiltInRule values that implement the trait. Lookup goes through a
new find_matching_rule helper that takes a custom_rules slice and a
use_built_in_rules toggle; this task wires the call site with an
empty slice and true so all 34 existing tests pass unchanged.

Stub src/config.rs created for Task 2.
EOF
)"
```

---

## Task 2: Add `OutputCase`, `NormalizerConfig`, `NormalizerConfigBuilder` in `src/config.rs`

**Files:**
- Modify: `src/config.rs` (full content)
- Modify: `src/lib.rs:8` (extend the re-export line)
- Modify: `src/tests.rs` (add 2 new tests)

**Goal of this task:** the data types + construction surface, with a builder roundtrip test. `normalize_str` is **not** yet wired to consult the config; that's Task 3.

- [ ] **Step 1: Write the failing builder roundtrip test**

Append to `src/tests.rs`:

```rust
// ── Task 2: builder + config ──────────────────────────────────────────────────

use super::{NormalizerConfig, NormalizerConfigBuilder, OutputCase};

#[test]
fn builder_round_trip_preserves_options() {
    let cfg: NormalizerConfig = NormalizerConfig::builder()
        .output_case(OutputCase::Lowercase)
        .apply_provider_rules(false)
        .resolve_domain_aliases(false)
        .use_built_in_rules(false)
        .build();

    // Test via behavior at the public surface: a config with all four
    // toggles flipped behaves differently from default for the same input.
    // This task doesn't yet wire those toggles into normalize_str, so the
    // behavioral assertion lives in Task 3 / 4. For Task 2, just prove
    // the value type round-trips through the builder by calling .clone().
    let _cloned = cfg.clone();
}

#[test]
fn default_config_is_constructible() {
    // Default::default() and NormalizerConfig::builder().build() must
    // both produce a config (the builder defaults match the value's
    // Default impl by construction).
    let _via_default: NormalizerConfig = NormalizerConfig::default();
    let _via_builder: NormalizerConfig = NormalizerConfig::builder().build();
}
```

- [ ] **Step 2: Run the test; expect a compile error**

Run: `cargo test --no-run 2>&1 | tail -10`

Expected: error E0432 or E0412 — `NormalizerConfig` / `NormalizerConfigBuilder` / `OutputCase` not found, because `src/config.rs` is empty.

- [ ] **Step 3: Replace `src/config.rs` with the implementation**

Overwrite the file with:

```rust
//! Configuration types for `Email::normalize_with`. Construct a
//! `NormalizerConfig` via `NormalizerConfig::default()` (current
//! library behavior) or via the builder for overrides.

use std::sync::Arc;

use crate::rules::ProviderRule;

/// Case of the normalized output. Default is `Uppercase` to match the
/// library's historical behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCase {
    Uppercase,
    Lowercase,
}

impl Default for OutputCase {
    fn default() -> Self {
        OutputCase::Uppercase
    }
}

/// All knobs controlling `Email::normalize_with`. Build one with
/// `NormalizerConfig::builder()` or call `NormalizerConfig::default()`
/// for the library's historical behavior.
#[derive(Clone)]
pub struct NormalizerConfig {
    output_case: OutputCase,
    apply_provider_rules: bool,
    resolve_domain_aliases: bool,
    custom_rules: Vec<Arc<dyn ProviderRule>>,
    use_built_in_rules: bool,
}

impl Default for NormalizerConfig {
    fn default() -> Self {
        Self {
            output_case: OutputCase::Uppercase,
            apply_provider_rules: true,
            resolve_domain_aliases: true,
            custom_rules: Vec::new(),
            use_built_in_rules: true,
        }
    }
}

impl NormalizerConfig {
    /// Start building a non-default `NormalizerConfig`.
    pub fn builder() -> NormalizerConfigBuilder {
        NormalizerConfigBuilder::default()
    }

    pub(crate) fn output_case(&self) -> OutputCase {
        self.output_case
    }

    pub(crate) fn apply_provider_rules(&self) -> bool {
        self.apply_provider_rules
    }

    pub(crate) fn resolve_domain_aliases(&self) -> bool {
        self.resolve_domain_aliases
    }

    pub(crate) fn custom_rules(&self) -> &[Arc<dyn ProviderRule>] {
        &self.custom_rules
    }

    pub(crate) fn use_built_in_rules(&self) -> bool {
        self.use_built_in_rules
    }
}

/// Builder for `NormalizerConfig`. Each setter consumes `self` and
/// returns `Self` for chaining; finalize with `.build()`.
pub struct NormalizerConfigBuilder {
    output_case: OutputCase,
    apply_provider_rules: bool,
    resolve_domain_aliases: bool,
    custom_rules: Vec<Arc<dyn ProviderRule>>,
    use_built_in_rules: bool,
}

impl Default for NormalizerConfigBuilder {
    fn default() -> Self {
        Self {
            output_case: OutputCase::Uppercase,
            apply_provider_rules: true,
            resolve_domain_aliases: true,
            custom_rules: Vec::new(),
            use_built_in_rules: true,
        }
    }
}

impl NormalizerConfigBuilder {
    /// Set the case of the normalized output.
    pub fn output_case(mut self, c: OutputCase) -> Self {
        self.output_case = c;
        self
    }

    /// When `false`, no local-part transformations are applied (matched
    /// rules' `transform_local` is skipped and the `DEFAULT_RULE`
    /// fallback is skipped).
    pub fn apply_provider_rules(mut self, on: bool) -> Self {
        self.apply_provider_rules = on;
        self
    }

    /// When `false`, the matched rule's `canonical_domain` is ignored
    /// and the original lowercased domain is kept.
    pub fn resolve_domain_aliases(mut self, on: bool) -> Self {
        self.resolve_domain_aliases = on;
        self
    }

    /// Append a custom rule. Custom rules are checked before built-ins
    /// during normalization, so a custom rule for a domain handled by
    /// the built-ins shadows the built-in rule.
    pub fn add_rule<R: ProviderRule + 'static>(mut self, rule: R) -> Self {
        self.custom_rules.push(Arc::new(rule));
        self
    }

    /// When `false`, the built-in rules (Gmail, iCloud, Outlook, Yahoo)
    /// are skipped; only custom rules apply. Combine with `add_rule` to
    /// replace built-ins entirely.
    pub fn use_built_in_rules(mut self, on: bool) -> Self {
        self.use_built_in_rules = on;
        self
    }

    /// Finalize the config.
    pub fn build(self) -> NormalizerConfig {
        NormalizerConfig {
            output_case: self.output_case,
            apply_provider_rules: self.apply_provider_rules,
            resolve_domain_aliases: self.resolve_domain_aliases,
            custom_rules: self.custom_rules,
            use_built_in_rules: self.use_built_in_rules,
        }
    }
}
```

- [ ] **Step 4: Extend re-export in `src/lib.rs`**

Change the re-export line in `src/lib.rs` from:
```rust
pub use rules::ProviderRule;
```
to:
```rust
pub use config::{NormalizerConfig, NormalizerConfigBuilder, OutputCase};
pub use rules::ProviderRule;
```

- [ ] **Step 5: Run tests; expect 36 passing (34 existing + 2 new)**

Run: `cargo test 2>&1 | tail -5`

Expected: `test result: ok. 36 passed; 0 failed`.

- [ ] **Step 6: Run clippy; expect clean**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`

Expected: no warnings. Possible nag: clippy may suggest `#[derive(Default)]` on `NormalizerConfigBuilder` since all its fields have `Default` — leave the manual impl because three booleans need to default to `true`, not `false` (which is what `bool::default()` gives). The manual impl is correct.

- [ ] **Step 7: Commit**

```bash
git add src/config.rs src/lib.rs src/tests.rs
git commit -m "$(cat <<'EOF'
Add NormalizerConfig, NormalizerConfigBuilder, OutputCase

Five-field config value with private fields and pub(crate) accessors.
Builder is a separate type with one setter per field plus add_rule
(generic, takes any ProviderRule + 'static) and use_built_in_rules
(bool toggle). Default impls on both the config and the builder
match the library's historical behavior (Uppercase, all toggles on,
no custom rules).

normalize_str doesn't yet consult the config — that's Task 3.
EOF
)"
```

---

## Task 3: Wire the three toggle options into `normalize_str`; add `Email::normalize_with`

**Files:**
- Modify: `src/lib.rs` (rewrite `normalize_str`; add `Email::normalize_with`)
- Modify: `src/tests.rs` (add 5 new tests)

**Goal of this task:** `Email::normalize_with(&NormalizerConfig)` is the new entry point. `Email::normalize()` becomes a one-liner delegating to it with `NormalizerConfig::default()`. `normalize_str` consults the config's three toggle options (output_case, apply_provider_rules, resolve_domain_aliases). Custom rules are **not** yet wired (Task 4 does that).

- [ ] **Step 1: Write the 5 failing tests**

Append to `src/tests.rs`:

```rust
// ── Task 3: normalize_with + toggle options ───────────────────────────────────

#[test]
fn default_config_matches_legacy_normalize() {
    // normalize_with(&NormalizerConfig::default()) must produce the
    // exact same output as the historical Email::normalize() for a
    // representative set of inputs.
    let cases = [
        "Foo@randomdomain.io",
        "foo-bar@Yahoo.CO.UK",
        "FOO@HOTMAIL.COM",
        "user@me.com",
        "S.T.E.V.E+x@GoogleMail.com",
    ];
    let cfg = NormalizerConfig::default();
    for original in cases {
        let legacy = Email::from(original).normalize();
        let via_with = Email::from(original).normalize_with(&cfg);
        assert_eq!(
            legacy.as_ref().map(|n| n.as_str()),
            via_with.as_ref().map(|n| n.as_str()),
            "mismatch for {original:?}"
        );
    }
}

#[test]
fn output_case_lowercase_produces_lowercase() {
    let cfg = NormalizerConfig::builder()
        .output_case(OutputCase::Lowercase)
        .build();
    assert_eq!(
        Email::from("S.T.E.V.E+x@GoogleMail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("steve@gmail.com"),
    );
}

#[test]
fn apply_provider_rules_false_keeps_local_intact() {
    // Gmail input: rule would normally strip plus and dots; with the
    // toggle off, the local part stays as the lowercased original.
    // Alias resolution still runs.
    let cfg = NormalizerConfig::builder()
        .apply_provider_rules(false)
        .build();
    assert_eq!(
        Email::from("s.t.e.v.e+x@googlemail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("S.T.E.V.E+X@GMAIL.COM"),
    );
}

#[test]
fn resolve_domain_aliases_false_keeps_alias_domain() {
    let cfg = NormalizerConfig::builder()
        .resolve_domain_aliases(false)
        .build();
    assert_eq!(
        Email::from("foo@googlemail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@GOOGLEMAIL.COM"),
    );
}

#[test]
fn apply_provider_rules_false_disables_default_rule_too() {
    // Unmatched domain ('randomdomain.io'): default rule would normally
    // strip plus. With the toggle off, plus is kept.
    let cfg = NormalizerConfig::builder()
        .apply_provider_rules(false)
        .build();
    assert_eq!(
        Email::from("foo+tag@randomdomain.io")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO+TAG@RANDOMDOMAIN.IO"),
    );
}
```

- [ ] **Step 2: Run tests; expect compile error**

Run: `cargo test --no-run 2>&1 | tail -10`

Expected: error E0599 — `normalize_with` method not found on `Email`.

- [ ] **Step 3: Rewrite `normalize_str` and add `Email::normalize_with`**

In `src/lib.rs`, replace the entire current `normalize_str` function with:

```rust
fn normalize_str(email: &str, config: &NormalizerConfig) -> Option<String> {
    let trimmed = email.trim();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }

    let mut local = local.to_ascii_lowercase();
    let mut domain = domain.to_ascii_lowercase();

    let matched = rules::find_matching_rule(&domain, &[], config.use_built_in_rules());

    if let Some(rule) = matched {
        if config.resolve_domain_aliases() {
            domain = rule.canonical_domain().to_string();
        }
        if config.apply_provider_rules() {
            local = rule.transform_local(&local);
        }
    } else if config.apply_provider_rules() {
        local = rules::DEFAULT_RULE.transform_local(&local);
    }

    if local.is_empty() {
        return None;
    }

    let joined = format!("{local}@{domain}");
    Some(match config.output_case() {
        OutputCase::Uppercase => joined.to_ascii_uppercase(),
        OutputCase::Lowercase => joined,
    })
}
```

Note: the `custom_rules` slice passed to `find_matching_rule` is still `&[]` — Task 4 changes that to `config.custom_rules()`.

In the same file, replace the existing `Email::normalize` impl:

```rust
impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The only public constructor for `NormalizedEmail`.
    pub fn normalize(&self) -> Option<NormalizedEmail> {
        normalize_str(&self.0).map(NormalizedEmail)
    }
}
```

with:

```rust
impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Normalize using the library's default configuration. Equivalent
    /// to `self.normalize_with(&NormalizerConfig::default())`.
    pub fn normalize(&self) -> Option<NormalizedEmail> {
        self.normalize_with(&NormalizerConfig::default())
    }

    /// Normalize using the given configuration. The only public
    /// constructor for `NormalizedEmail`.
    pub fn normalize_with(&self, config: &NormalizerConfig) -> Option<NormalizedEmail> {
        normalize_str(&self.0, config).map(NormalizedEmail)
    }
}
```

- [ ] **Step 4: Run tests; expect 41 passing (34 + 2 from Task 2 + 5 new)**

Run: `cargo test 2>&1 | tail -10`

Expected: `test result: ok. 41 passed; 0 failed`. In particular:
- All 34 original tests still pass because `Default` matches historical behavior.
- The 2 builder tests from Task 2 still pass.
- The 5 new toggle tests pass.

If any of the original 34 fail, the new config path drifted from historical behavior — debug before proceeding.

- [ ] **Step 5: Run clippy; expect clean**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`

Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/tests.rs
git commit -m "$(cat <<'EOF'
Wire NormalizerConfig toggles into Email::normalize_with

Email::normalize_with(&NormalizerConfig) is the new entry point;
Email::normalize() is now a one-liner delegating to it with
Default. normalize_str consults output_case, apply_provider_rules,
and resolve_domain_aliases. Custom rules still passed as &[];
Task 4 wires those.
EOF
)"
```

---

## Task 4: Wire custom rules into `normalize_str`; add tests for stack and replace modes

**Files:**
- Modify: `src/lib.rs:~20` (change one line: the `find_matching_rule` call site)
- Modify: `src/tests.rs` (add 5 new tests)

**Goal of this task:** complete the feature by passing `config.custom_rules()` into `find_matching_rule`, then prove the stack and replace flows work end-to-end.

- [ ] **Step 1: Write the 5 failing tests**

Append to `src/tests.rs`:

```rust
// ── Task 4: custom rules (stack + replace) ────────────────────────────────────

use crate::ProviderRule;

struct MyCorpRule;
impl ProviderRule for MyCorpRule {
    fn matches_domain(&self, d: &str) -> bool {
        d == "mycorp.com"
    }
    fn canonical_domain(&self) -> &str {
        "mycorp.com"
    }
    fn transform_local(&self, local: &str) -> String {
        // mycorp uses underscore as subaddress separator.
        local.split('_').next().unwrap_or("").to_string()
    }
}

/// A custom rule that *shadows* the built-in gmail rule by matching
/// gmail.com but applying a different local transform (no-op).
struct GmailVerbatimRule;
impl ProviderRule for GmailVerbatimRule {
    fn matches_domain(&self, d: &str) -> bool {
        d == "gmail.com" || d == "googlemail.com"
    }
    fn canonical_domain(&self) -> &str {
        "gmail.com"
    }
    fn transform_local(&self, local: &str) -> String {
        local.to_string()
    }
}

#[test]
fn custom_rule_stacks_on_built_ins() {
    let cfg = NormalizerConfig::builder().add_rule(MyCorpRule).build();
    // Custom rule handles mycorp.com (built-ins don't know about it).
    assert_eq!(
        Email::from("alice_promo@mycorp.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("ALICE@MYCORP.COM"),
    );
    // Built-ins still work alongside the custom rule.
    assert_eq!(
        Email::from("foo+bar@gmail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@GMAIL.COM"),
    );
}

#[test]
fn custom_rule_shadows_built_in() {
    let cfg = NormalizerConfig::builder()
        .add_rule(GmailVerbatimRule)
        .build();
    // GmailVerbatimRule is checked before the built-in gmail rule, so
    // plus and dots are preserved.
    assert_eq!(
        Email::from("s.t.e.v.e+x@gmail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("S.T.E.V.E+X@GMAIL.COM"),
    );
}

#[test]
fn use_built_in_rules_false_with_only_custom_rules() {
    // Replace mode: built-ins off, only MyCorpRule is in play.
    let cfg = NormalizerConfig::builder()
        .use_built_in_rules(false)
        .add_rule(MyCorpRule)
        .build();
    // Mycorp still works (the custom rule matches).
    assert_eq!(
        Email::from("alice_promo@mycorp.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("ALICE@MYCORP.COM"),
    );
    // Gmail no longer gets its built-in treatment: nothing matches,
    // DEFAULT_RULE fires (strip plus), domain kept as-is.
    assert_eq!(
        Email::from("foo+bar@gmail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@GMAIL.COM"),
    );
    // googlemail.com no longer maps to gmail.com.
    assert_eq!(
        Email::from("foo@googlemail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@GOOGLEMAIL.COM"),
    );
}

#[test]
fn use_built_in_rules_false_with_no_custom_rules_falls_through_to_default() {
    // No rules at all — only DEFAULT_RULE applies (strips plus).
    let cfg = NormalizerConfig::builder()
        .use_built_in_rules(false)
        .build();
    assert_eq!(
        Email::from("s.t.e.v.e+x@gmail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("S.T.E.V.E@GMAIL.COM"),
    );
}

#[test]
fn idempotency_holds_with_custom_rule_and_lowercase_output() {
    let cfg = NormalizerConfig::builder()
        .output_case(OutputCase::Lowercase)
        .add_rule(MyCorpRule)
        .build();
    let original = Email::from("Alice_Promo@MyCorp.com");
    let once = original.normalize_with(&cfg).expect("first normalize");
    let twice = Email::from(once.as_str())
        .normalize_with(&cfg)
        .expect("second normalize");
    assert_eq!(once.as_str(), twice.as_str());
    assert_eq!(once.as_str(), "alice@mycorp.com");
}
```

- [ ] **Step 2: Run tests; expect failures**

Run: `cargo test 2>&1 | tail -20`

Expected: the new tests fail because `normalize_str` is still calling `find_matching_rule(&domain, &[], config.use_built_in_rules())`. The custom rules are in the config but ignored.

Specifically:
- `custom_rule_stacks_on_built_ins` — fails on the mycorp.com assertion (no rule matches, DEFAULT_RULE applies, output is `ALICE_PROMO@MYCORP.COM`).
- `custom_rule_shadows_built_in` — fails (built-in gmail rule applies, output is `STEVE@GMAIL.COM`).
- `use_built_in_rules_false_with_only_custom_rules` — fails on the mycorp assertion (no rule matches).
- `use_built_in_rules_false_with_no_custom_rules_falls_through_to_default` — actually this one might pass already, since the existing wiring already honors `use_built_in_rules` via the third argument to `find_matching_rule`. Confirm: with built-ins off and no custom rules, no rule matches, DEFAULT_RULE strips plus from `s.t.e.v.e+x` → `s.t.e.v.e`, domain kept as `gmail.com`, uppercased → `S.T.E.V.E@GMAIL.COM`. That matches the test's expectation. So this test passes already in Task 3's wiring. Leaving it in regardless — it's worth having as a regression check.
- `idempotency_holds_with_custom_rule_and_lowercase_output` — fails (custom rule ignored, output is `alice_promo@mycorp.com`).

- [ ] **Step 3: Wire `custom_rules` into the call site**

In `src/lib.rs`, change the line:

```rust
    let matched = rules::find_matching_rule(&domain, &[], config.use_built_in_rules());
```

to:

```rust
    let matched = rules::find_matching_rule(
        &domain,
        config.custom_rules(),
        config.use_built_in_rules(),
    );
```

That's the entire implementation change.

- [ ] **Step 4: Run tests; expect 46 passing (34 + 2 + 5 + 5)**

Run: `cargo test 2>&1 | tail -10`

Expected: `test result: ok. 46 passed; 0 failed`.

- [ ] **Step 5: Run clippy; expect clean**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`

Expected: no warnings.

- [ ] **Step 6: Run rustdoc; expect clean**

Run: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps 2>&1 | tail -10`

Expected: builds without warnings. Doc comments on public types render correctly.

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/tests.rs
git commit -m "$(cat <<'EOF'
Wire custom rules through normalize_str; add stack/replace tests

normalize_str now passes config.custom_rules() into
find_matching_rule. Tests cover: stacking a custom rule on top of
built-ins, shadowing a built-in by matching the same domain,
replace mode (use_built_in_rules(false) + add_rule), DEFAULT_RULE
fallthrough when no rule matches and built-ins are disabled,
idempotency under combined lowercase + custom rule.
EOF
)"
```

---

## Spec coverage self-check

| Spec requirement | Task / Step |
|---|---|
| Public `ProviderRule` trait with `matches_domain` / `canonical_domain` / `transform_local` | Task 1, Step 2 |
| `Send + Sync` + object-safe trait | Task 1, Step 2 |
| Private `BuiltInRule` struct implementing the trait; existing rule data preserved | Task 1, Step 2 |
| `BUILT_IN_RULES: &[BuiltInRule]` static; `DEFAULT_RULE` is a `BuiltInRule` | Task 1, Step 2 |
| `find_matching_rule(domain, custom_rules, use_built_in_rules)` helper, custom rules checked first | Task 1, Step 2 |
| `pub use rules::ProviderRule` re-export from lib.rs | Task 1, Step 3 |
| `OutputCase` enum (Uppercase, Lowercase) with Default = Uppercase | Task 2, Step 3 |
| `NormalizerConfig` with 5 private fields and `Default` matching historical behavior | Task 2, Step 3 |
| `pub(crate)` getter methods on `NormalizerConfig` | Task 2, Step 3 |
| `NormalizerConfigBuilder` with 5 setter methods + `build()` + manual `Default` for the bool defaults | Task 2, Step 3 |
| `add_rule<R: ProviderRule + 'static>(rule: R)` — generic, wraps in `Arc` | Task 2, Step 3 |
| `use_built_in_rules(bool)` toggle | Task 2, Step 3 |
| `pub use config::{NormalizerConfig, NormalizerConfigBuilder, OutputCase}` re-export | Task 2, Step 4 |
| `Email::normalize_with(&NormalizerConfig) -> Option<NormalizedEmail>` | Task 3, Step 3 |
| `Email::normalize()` delegates to `normalize_with(&Default::default())` | Task 3, Step 3 |
| `normalize_str` signature takes `&NormalizerConfig` | Task 3, Step 3 |
| `output_case` honored | Task 3, Step 3 |
| `apply_provider_rules` honored (including default fallback gating) | Task 3, Step 3 |
| `resolve_domain_aliases` honored | Task 3, Step 3 |
| Custom rules consulted in `normalize_str` | Task 4, Step 3 |
| `Clone` derived on `NormalizerConfig` (cheap, Arc-backed) | Task 2, Step 3 |
| Tests: existing 34 stay green; new tests for every option | Tasks 1, 2, 3, 4 |
| Tests: builder roundtrip | Task 2, Step 1 |
| Tests: default config equivalence with legacy `normalize()` | Task 3, Step 1 |
| Tests: each toggle option | Task 3, Step 1 |
| Tests: stack mode + shadow + replace mode | Task 4, Step 1 |
| Tests: idempotency under lowercase + custom rule | Task 4, Step 1 |

All spec requirements covered.
