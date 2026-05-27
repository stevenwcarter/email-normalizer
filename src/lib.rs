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
