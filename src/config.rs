//! Configuration types for `Email::normalize_with`. Construct a
//! `NormalizerConfig` via `NormalizerConfig::default()` (current
//! library behavior) or via the builder for overrides.

use std::sync::Arc;

use crate::rules::ProviderRule;

/// Case of the normalized output. Default is `Uppercase` to match the
/// library's historical behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputCase {
    /// Output the normalized email in uppercase (e.g. `ALICE@GMAIL.COM`).
    #[default]
    Uppercase,
    /// Output the normalized email in lowercase (e.g. `alice@gmail.com`).
    Lowercase,
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
    /// Start building a `NormalizerConfig`. All options default to the
    /// library's historical behavior — call setters only for what you
    /// want to override.
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
    pub fn add_rule<R: ProviderRule>(mut self, rule: R) -> Self {
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
