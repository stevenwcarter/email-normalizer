# NormalizerConfig + ProviderRule trait + builder API

**Date:** 2026-05-27
**Status:** Approved
**Scope:** `src/lib.rs`, `src/rules.rs`, new `src/config.rs`, new tests in `src/tests.rs`

## Goal

Open up the email normalizer to consumers who want non-default behavior.
Today, `Email::normalize()` is hardcoded to (a) uppercase output, (b)
apply built-in provider rules, (c) resolve built-in domain aliases.
After this change, the same defaults are preserved, but consumers can
override them via a `NormalizerConfig` value constructed with a typical
builder pattern, and can plug in their own normalization rules via a
public `ProviderRule` trait — stacked on top of the built-ins or
replacing them entirely.

## Why

- Different products canonicalize email differently. Some prefer
  lowercase to match the SMTP RFC's case-sensitivity rules in spirit;
  others need uppercase for legacy unique-constraint columns.
- Users with private mail domains (corporate ESPs, niche providers
  like Fastmail or ProtonMail) need a way to teach the library their
  rules without forking. A public trait is the canonical Rust way to
  do that.
- "Apply provider rules at all" and "resolve domain aliases" are
  cheap to expose alongside case (they're already independent knobs in
  the implementation). Exposing them now establishes the option-bag
  shape without paying for it later.

## Non-goals

- No deprecation of `Email::normalize()`. The existing signature stays
  and continues to work; it now delegates to
  `normalize_with(&NormalizerConfig::default())`. All 34 existing
  tests pass unchanged.
- No fluent setter API on `NormalizerConfig` itself (no `with_*`
  methods on the value type). The builder is the dedicated
  construction surface; the value type is just data.
- No changes to `Email` or `NormalizedEmail` newtypes' public surface
  beyond the new `normalize_with` method.
- No serde derives on `NormalizerConfig` / `OutputCase` /
  `NormalizerConfigBuilder`. Out of scope for this iteration. If a
  consumer needs to serialize a config they can build one programmatically.
- No `derive_builder` crate dependency. The builder is hand-rolled —
  five fields, two methods that aren't simple setters; the boilerplate
  is small enough to read.
- No way to override the catch-all `DEFAULT_RULE` (the rule applied
  when no provider rule matches). Users who want different fallback
  behavior add a custom rule that matches their target domains
  explicitly, or set `apply_provider_rules(false)` to skip all local-
  part transformation entirely.

## Design

### Module layout

| File | Contents |
|---|---|
| `src/lib.rs` | `Email`, `NormalizedEmail`, `normalize_str(email, &NormalizerConfig)`, re-exports of public types from `config.rs` and `rules.rs`. |
| `src/config.rs` | NEW. `pub enum OutputCase`, `pub struct NormalizerConfig`, `pub struct NormalizerConfigBuilder`. |
| `src/rules.rs` | `pub trait ProviderRule`, private `struct BuiltInRule` (the old `ProviderRule` struct renamed), `BUILT_IN_RULES: &[BuiltInRule]`, `DEFAULT_RULE` (a `BuiltInRule` constant), helper `find_matching_rule(domain, custom_rules, use_built_in_rules) -> Option<&dyn ProviderRule>`. |
| `src/tests.rs` | Existing 34 tests stay. New tests added for each builder method and each new behavior. |

### Public trait: `ProviderRule`

```rust
pub trait ProviderRule: Send + Sync {
    /// True if this rule applies to `domain` (already lowercased).
    /// A rule matches both its canonical domain AND any aliases it
    /// owns — the rule decides what "matches" means.
    fn matches_domain(&self, domain: &str) -> bool;

    /// The canonical form of the domain — what `domain` rewrites to
    /// when `matches_domain` returned true. May be the same string
    /// the domain already was.
    fn canonical_domain(&self) -> &str;

    /// Transform the local part. Return the new local. An empty
    /// return value is checked by the caller and causes the whole
    /// email to be rejected (returns `None` from `normalize_with`).
    fn transform_local(&self, local: &str) -> String;
}
```

Properties:

- `Send + Sync`: rules will be held in `Arc<dyn ProviderRule>` and
  shared across threads. `Send + Sync` is the minimum for that.
- Object-safe: no generic methods, no `Self` in return positions, no
  associated types. `dyn ProviderRule` works.
- Three methods, not one combined `apply`. Keeps domain matching and
  local transformation independently overridable, and lets the alias-
  resolution toggle in `NormalizerConfig` skip just the domain rewrite
  without skipping the local transform (or vice versa).

### Built-in rules: private `BuiltInRule` struct

The existing `pub struct ProviderRule` in `rules.rs` is renamed
`BuiltInRule` (private to the crate) and updated to implement the new
public `ProviderRule` trait:

```rust
struct BuiltInRule {
    canonical_domain: &'static str,
    aliases: &'static [&'static str],
    strip_plus: bool,
    strip_dash: bool,
    strip_dots: bool,
}

impl ProviderRule for BuiltInRule {
    fn matches_domain(&self, d: &str) -> bool {
        d == self.canonical_domain || self.aliases.contains(&d)
    }
    fn canonical_domain(&self) -> &str { self.canonical_domain }
    fn transform_local(&self, local: &str) -> String {
        let mut s = local.to_string();
        if self.strip_plus && let Some(idx) = s.find('+') {
            s.truncate(idx);
        }
        if self.strip_dash && let Some(idx) = s.find('-') {
            s.truncate(idx);
        }
        if self.strip_dots {
            s = s.replace('.', "");
        }
        s
    }
}
```

`PROVIDER_RULES: &[BuiltInRule]` and `DEFAULT_RULE: BuiltInRule`
constants stay as `static` data — zero allocation, zero overhead for
the default path. They are renamed:
- `PROVIDER_RULES` → `BUILT_IN_RULES`
- `DEFAULT_RULE` keeps its name

The existing free functions `resolve_domain` and `rule_for_domain` are
**replaced** by a single helper:

```rust
pub(crate) fn find_matching_rule<'a>(
    domain: &str,
    custom_rules: &'a [Arc<dyn ProviderRule>],
    use_built_in_rules: bool,
) -> Option<&'a dyn ProviderRule>
```

It iterates custom rules first, then built-ins (if `use_built_in_rules`),
returning the first match. Custom rules can shadow built-ins by
matching the same domain.

### Public enum: `OutputCase`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCase {
    Uppercase,
    Lowercase,
}

impl Default for OutputCase {
    fn default() -> Self { OutputCase::Uppercase }  // current behavior
}
```

### Public struct: `NormalizerConfig`

```rust
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
    pub fn builder() -> NormalizerConfigBuilder {
        NormalizerConfigBuilder::default()
    }

    pub(crate) fn output_case(&self) -> OutputCase { self.output_case }
    pub(crate) fn apply_provider_rules(&self) -> bool { self.apply_provider_rules }
    pub(crate) fn resolve_domain_aliases(&self) -> bool { self.resolve_domain_aliases }
    pub(crate) fn custom_rules(&self) -> &[Arc<dyn ProviderRule>] { &self.custom_rules }
    pub(crate) fn use_built_in_rules(&self) -> bool { self.use_built_in_rules }
}
```

- All fields private. Builder is the only construction path beyond
  `Default`.
- Getters are `pub(crate)` so `normalize_str` in `lib.rs` can read
  them without exposing them as public surface (consumers don't need
  to introspect a config they built).
- Derives `Clone` (cheap because rules are `Arc`).
- No `Debug` derive — `Arc<dyn ProviderRule>` has no `Debug` bound on
  the trait, so it would require either adding `: Debug` to the trait
  bound (intrusive on user impls) or a manual `Debug` impl. Skip for now.

### Public struct: `NormalizerConfigBuilder`

```rust
#[derive(Default)]
pub struct NormalizerConfigBuilder {
    // Mirrors NormalizerConfig fields with the same defaults.
    output_case: OutputCase,
    apply_provider_rules: bool,  // initialized to true via the Default below
    resolve_domain_aliases: bool,
    custom_rules: Vec<Arc<dyn ProviderRule>>,
    use_built_in_rules: bool,
}

// Manual Default because three bools need to be `true` not `false`.
impl Default for NormalizerConfigBuilder {
    fn default() -> Self {
        Self {
            output_case: OutputCase::default(),
            apply_provider_rules: true,
            resolve_domain_aliases: true,
            custom_rules: Vec::new(),
            use_built_in_rules: true,
        }
    }
}

impl NormalizerConfigBuilder {
    pub fn output_case(mut self, c: OutputCase) -> Self {
        self.output_case = c; self
    }
    pub fn apply_provider_rules(mut self, on: bool) -> Self {
        self.apply_provider_rules = on; self
    }
    pub fn resolve_domain_aliases(mut self, on: bool) -> Self {
        self.resolve_domain_aliases = on; self
    }
    pub fn add_rule<R: ProviderRule + 'static>(mut self, rule: R) -> Self {
        self.custom_rules.push(Arc::new(rule)); self
    }
    pub fn use_built_in_rules(mut self, on: bool) -> Self {
        self.use_built_in_rules = on; self
    }
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

**Stacking** (default): `NormalizerConfig::builder().add_rule(MyRule).build()`
— custom rules check first, built-ins still on.

**Replacement**: `NormalizerConfig::builder().use_built_in_rules(false).add_rule(MyRule).build()`
— custom rules only, built-ins skipped. (`DEFAULT_RULE` still applies
to unmatched domains, gated by `apply_provider_rules`.)

### Changes to `Email`

```rust
impl Email {
    pub fn normalize(&self) -> Option<NormalizedEmail> {
        self.normalize_with(&NormalizerConfig::default())
    }

    pub fn normalize_with(&self, config: &NormalizerConfig) -> Option<NormalizedEmail> {
        normalize_str(&self.0, config).map(NormalizedEmail)
    }
}
```

`normalize` becomes a one-liner. Existing callers and tests work unchanged.

### Rewritten `normalize_str`

```rust
fn normalize_str(email: &str, config: &NormalizerConfig) -> Option<String> {
    let trimmed = email.trim();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }

    let mut local = local.to_ascii_lowercase();
    let mut domain = domain.to_ascii_lowercase();

    let matched = rules::find_matching_rule(
        &domain,
        config.custom_rules(),
        config.use_built_in_rules(),
    );

    if let Some(rule) = matched {
        if config.resolve_domain_aliases() {
            domain = rule.canonical_domain().to_string();
        }
        if config.apply_provider_rules() {
            local = rule.transform_local(&local);
        }
    } else if config.apply_provider_rules() {
        // No specific rule matched; fall back to DEFAULT_RULE.
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

### Re-exports in `lib.rs`

```rust
pub use config::{NormalizerConfig, NormalizerConfigBuilder, OutputCase};
pub use rules::ProviderRule;
```

Consumers can write `use email_normalizer::{Email, NormalizerConfig, OutputCase, ProviderRule};` — flat namespace.

## Behavior matrix

Validates the design against every option combination on a known input:

| Input | Config | Output |
|---|---|---|
| `s.t.e.v.e+x@gmail.com` | Default | `STEVE@GMAIL.COM` |
| `s.t.e.v.e+x@gmail.com` | `output_case(Lowercase)` | `steve@gmail.com` |
| `s.t.e.v.e+x@gmail.com` | `apply_provider_rules(false)` | `S.T.E.V.E+X@GMAIL.COM` |
| `s.t.e.v.e+x@gmail.com` | `resolve_domain_aliases(false)` | `STEVE@GMAIL.COM` (canonical = alias here) |
| `foo@googlemail.com` | Default | `FOO@GMAIL.COM` |
| `foo@googlemail.com` | `resolve_domain_aliases(false)` | `FOO@GOOGLEMAIL.COM` |
| `foo+x@hotmail.co.uk` | `apply_provider_rules(false)` | `FOO+X@OUTLOOK.COM` (alias resolved, local kept) |
| `foo+x@hotmail.co.uk` | `resolve_domain_aliases(false)` | `FOO@HOTMAIL.CO.UK` (DEFAULT_RULE strips plus on the still-alias domain because nothing matched after alias resolution was skipped — wait: the matched rule still applies since `find_matching_rule` doesn't care about the alias flag. The matched rule's local transform still runs. So this is `FOO@HOTMAIL.CO.UK`.) |
| `alice_promo@mycorp.com` | `add_rule(MyCorpRule)` (stack) | `ALICE@MYCORP.COM` |
| `something+x@gmail.com` | `add_rule(MyCorpRule).use_built_in_rules(false)` (replace) | `SOMETHING@GMAIL.COM` |

The last row's derivation, since it's subtle: with `use_built_in_rules(false)` and only `MyCorpRule` added, `find_matching_rule("gmail.com", &[MyCorpRule], false)` returns `None` (MyCorpRule only matches `mycorp.com`). Built-ins are skipped. The `else if config.apply_provider_rules()` branch then runs (apply_provider_rules defaults to `true`), applying `DEFAULT_RULE.transform_local` which strips plus. Final: `SOMETHING@GMAIL.COM`. The plan will include an explicit test for this case.

## Testing strategy

Two layers, both in `src/tests.rs`:

**Layer 1 — regression: all 34 existing tests stay green.** They use
`Email::normalize()`, which now delegates to
`normalize_with(&NormalizerConfig::default())`. Equivalence holds by
the design of `Default`.

**Layer 2 — new tests** (one per design surface):

1. `default_config_round_trip` — `Email::normalize()` ==
   `Email::normalize_with(&NormalizerConfig::default())` on a
   representative set (covers the "Default option" requirement).
2. `output_case_lowercase` — `output_case(Lowercase)` produces
   lowercase output for a known input.
3. `apply_provider_rules_false_skips_local_transform` — plus is kept,
   dots are kept.
4. `resolve_domain_aliases_false_keeps_alias_domain` —
   `foo@googlemail.com` stays at `googlemail.com`.
5. `apply_provider_rules_false_still_resolves_aliases` —
   `foo@googlemail.com` becomes `FOO@GMAIL.COM`.
6. `custom_rule_stacks_on_built_ins` — a `MyCorpRule` for
   `mycorp.com` works; gmail still works.
7. `custom_rule_shadows_built_in` — a custom rule for `gmail.com`
   that uppercases instead of stripping plus shadows the built-in.
8. `use_built_in_rules_false_disables_built_ins` — gmail input
   doesn't get plus-stripped because no rule matches AND
   `DEFAULT_RULE` only kicks in when nothing matches AND
   `apply_provider_rules` is true; verify the DEFAULT fallback DOES
   still apply by default.
9. `use_built_in_rules_false_with_custom_rule_only` — only the
   custom rule applies; unmatched domains hit DEFAULT.
10. `builder_round_trip` — `NormalizerConfig::builder().output_case(...).build()` produces
    a config whose accessors return what was set.
11. `arc_clone_is_cheap` — `NormalizerConfig::clone()` succeeds and
    cloned config produces identical output (lightweight check that
    Clone is wired through Arc).
12. `idempotency_under_lowercase_option` — normalizing a normalized
    email is still idempotent under `output_case(Lowercase)`.
13. `idempotency_with_custom_rule` — same, with a custom rule.

## Risks

- **API churn**: this is the first significant public-API change to
  the library. Existing callers (`Email::normalize()`) keep working
  unchanged; new callers get the builder. Library is `0.1.0`, no
  external consumers known — low risk of breakage.
- **`Arc<dyn ProviderRule>` overhead**: one heap allocation per custom
  rule, at config construction time. Built-in rules stay as `&'static`
  data through their dedicated `BUILT_IN_RULES` slice — no Arc cost
  for the default path. The custom-rules iteration in
  `find_matching_rule` is one extra Vec iteration; trivial.
- **Object safety**: trait must stay object-safe so `Arc<dyn ProviderRule>`
  works. Future trait additions need to preserve this — guard with a
  test that builds an `Arc<dyn ProviderRule>` from a custom impl.
- **`use_built_in_rules` naming**: the bool reads slightly oddly when
  chained (`.use_built_in_rules(false)`), but it's symmetric with the
  other toggles and the verb-first form ("use") makes intent obvious.
  Considered `without_built_in_rules()` (no-arg) but kept symmetric
  with the existing toggle pattern.

## Decisions captured from brainstorming

- Q1 (scope of options): "Case + provider rules + alias resolution"
  — three orthogonal toggles.
- User directive: "ensure the rules have a public trait so people
  can implement their own normalizer rules and pass them in. Add a
  method for stacking those rules easily on top of the default
  rules, or to replace them completely." Implemented via the public
  `ProviderRule` trait, `.add_rule()` (stack), and
  `.use_built_in_rules(false)` (replace mode).
