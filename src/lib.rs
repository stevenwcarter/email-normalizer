//! Provider-aware email normalization. Returns a canonical form
//! (uppercase by default; configurable via [`NormalizerConfig`])
//! suitable for unique-constraint enforcement and lookup.
//! Never serialized to the frontend.

mod config;
mod rules;

#[cfg(test)]
mod tests;

pub use config::{NormalizerConfig, NormalizerConfigBuilder, OutputCase};
pub use rules::ProviderRule;

fn normalize_str(email: &str, config: &NormalizerConfig) -> Option<String> {
    let trimmed = email.trim();
    let (local, domain) = trimmed.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }

    let mut local = local.to_ascii_lowercase();
    let mut domain = domain.to_ascii_lowercase();

    let matched =
        rules::find_matching_rule(&domain, config.custom_rules(), config.use_built_in_rules());

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

/// Raw, user-provided email. No validation; typed wrapper around String.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Normalize using the library's default configuration. Equivalent
    /// to `self.normalize_with(&NormalizerConfig::default())`.
    pub fn normalize(&self) -> Option<NormalizedEmail> {
        self.normalize_with(&NormalizerConfig::default())
    }

    /// Normalize using the given configuration.
    pub fn normalize_with(&self, config: &NormalizerConfig) -> Option<NormalizedEmail> {
        normalize_str(&self.0, config).map(NormalizedEmail)
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
