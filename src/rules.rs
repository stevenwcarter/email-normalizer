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
/// Implementors must be `Send + Sync + 'static`. `Send + Sync` lets
/// rules be shared across threads through `Arc<dyn ProviderRule>`;
/// `'static` is required because `Arc<dyn Trait>` (with no explicit
/// lifetime) is sugar for `Arc<dyn Trait + 'static>`, and that is the
/// container the `NormalizerConfig` builder uses internally. The
/// trait is object-safe.
pub trait ProviderRule: Send + Sync + 'static {
    /// True if this rule applies to `domain` (already lowercased).
    /// A rule matches both its canonical domain and any aliases it owns.
    fn matches_domain(&self, domain: &str) -> bool;

    /// The canonical form of the domain — what `domain` rewrites to when
    /// `matches_domain` returned true.
    fn canonical_domain(&self) -> &str;

    /// Transform the local part. `local` arrives already lowercased
    /// (consistent with the `domain` parameter to `matches_domain`).
    /// Return value of `""` causes the caller to reject the whole
    /// email (the caller checks for emptiness).
    fn transform_local(&self, local: &str) -> String;
}

pub(crate) struct BuiltInRule {
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
