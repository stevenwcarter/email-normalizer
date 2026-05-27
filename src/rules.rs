//! Per-provider normalization rules. Each rule specifies how to fold the
//! local part of an address for a specific provider. The rule's
//! `canonical_domain` is what the domain is rewritten to when one of its
//! `aliases` matches.
//!
//! The `DEFAULT_RULE` is used when no provider rule matches. Its
//! `canonical_domain` is `""` — a sentinel meaning "don't rewrite the
//! domain"; the original (lowercased) domain is kept.

pub struct ProviderRule {
    pub canonical_domain: &'static str,
    pub aliases: &'static [&'static str],
    pub strip_plus: bool,
    pub strip_dash: bool,
    pub strip_dots: bool,
}

pub const PROVIDER_RULES: &[ProviderRule] = &[
    // Gmail / Google Workspace
    ProviderRule {
        canonical_domain: "gmail.com",
        aliases: &["googlemail.com"],
        strip_plus: true,
        strip_dash: false,
        strip_dots: true,
    },
    // Apple iCloud
    ProviderRule {
        canonical_domain: "icloud.com",
        aliases: &["me.com", "mac.com"],
        strip_plus: true,
        strip_dash: false,
        strip_dots: false,
    },
    // Microsoft (Outlook / Hotmail / Live / MSN, with common regional TLDs)
    ProviderRule {
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
    ProviderRule {
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

pub const DEFAULT_RULE: ProviderRule = ProviderRule {
    canonical_domain: "",
    aliases: &[],
    strip_plus: true,
    strip_dash: false,
    strip_dots: false,
};

/// Resolve `domain` (lowercased) to its canonical form by consulting the
/// alias tables in `PROVIDER_RULES`. Returns the original domain if no
/// alias matches.
pub fn resolve_domain(domain: &str) -> &str {
    for rule in PROVIDER_RULES {
        if rule.aliases.contains(&domain) {
            return rule.canonical_domain;
        }
    }
    domain
}

/// Return the rule for `canonical_domain` (already alias-resolved), or
/// `&DEFAULT_RULE` if no rule matches.
pub fn rule_for_domain(canonical_domain: &str) -> &'static ProviderRule {
    for rule in PROVIDER_RULES {
        if rule.canonical_domain == canonical_domain {
            return rule;
        }
    }
    &DEFAULT_RULE
}
