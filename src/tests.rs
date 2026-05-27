#![cfg(test)]

use super::Email;

#[test]
fn normalizes_basic_address_to_uppercase() {
    assert_eq!(
        Email::from("alice@example.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("ALICE@EXAMPLE.COM")
    );
}

#[test]
fn trims_whitespace_and_uppercases() {
    assert_eq!(
        Email::from("  Alice@Example.COM  ")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("ALICE@EXAMPLE.COM"),
    );
}

#[test]
fn rejects_input_with_no_at_sign() {
    assert!(Email::from("no-at-sign").normalize().is_none());
}

#[test]
fn rejects_empty_local_part() {
    assert!(Email::from("@nope.com").normalize().is_none());
}

#[test]
fn rejects_empty_domain() {
    assert!(Email::from("nope@").normalize().is_none());
}

#[test]
fn splits_on_last_at_sign() {
    // We don't *support* quoted local parts, but we don't crash on
    // multiple @s either — split on the last one.
    assert_eq!(
        Email::from("a@b@c.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("A@B@C.COM"),
    );
}

#[test]
fn default_rule_strips_plus_subaddress() {
    assert_eq!(
        Email::from("foo+tag@randomdomain.io")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@RANDOMDOMAIN.IO"),
    );
}

#[test]
fn default_rule_keeps_dash_in_local() {
    assert_eq!(
        Email::from("foo-bar@randomdomain.io")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO-BAR@RANDOMDOMAIN.IO"),
    );
}

#[test]
fn default_rule_keeps_dots_in_local() {
    assert_eq!(
        Email::from("foo.bar@randomdomain.io")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO.BAR@RANDOMDOMAIN.IO"),
    );
}

#[test]
fn rejects_when_plus_strip_leaves_empty_local() {
    // Local is purely a subaddress tag → after strip, empty → reject.
    assert!(Email::from("+only@randomdomain.io").normalize().is_none());
}

#[test]
fn gmail_strips_plus_subaddress() {
    assert_eq!(
        Email::from("something+other@gmail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("SOMETHING@GMAIL.COM"),
    );
}

#[test]
fn googlemail_alias_maps_to_gmail() {
    assert_eq!(
        Email::from("something@googlemail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("SOMETHING@GMAIL.COM"),
    );
}

#[test]
fn gmail_strips_dots_from_local() {
    assert_eq!(
        Email::from("s.t.e.v.e@gmail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("STEVE@GMAIL.COM"),
    );
}

#[test]
fn gmail_keeps_dash_in_local() {
    // Gmail does NOT treat `-` as a separator.
    assert_eq!(
        Email::from("john-doe@gmail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("JOHN-DOE@GMAIL.COM"),
    );
}

#[test]
fn gmail_compound_rule_application() {
    // Plus strip, dot strip, alias flatten — all at once.
    assert_eq!(
        Email::from("s.t.e.v.e+promo@googlemail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("STEVE@GMAIL.COM"),
    );
}

#[test]
fn yahoo_strips_dash_subaddress() {
    assert_eq!(
        Email::from("something-other@yahoo.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("SOMETHING@YAHOO.COM"),
    );
}

#[test]
fn yahoo_keeps_plus_in_local() {
    // Yahoo doesn't use `+` as a separator; keep it.
    assert_eq!(
        Email::from("foo+x@yahoo.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO+X@YAHOO.COM"),
    );
}

#[test]
fn yahoo_regional_tld_maps_to_yahoo_com() {
    assert_eq!(
        Email::from("foo-shop@yahoo.co.uk")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@YAHOO.COM"),
    );
}

#[test]
fn ymail_alias_maps_to_yahoo_com() {
    assert_eq!(
        Email::from("foo@ymail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@YAHOO.COM"),
    );
}

#[test]
fn rocketmail_alias_maps_to_yahoo_com() {
    assert_eq!(
        Email::from("foo@rocketmail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@YAHOO.COM"),
    );
}

#[test]
fn me_dot_com_maps_to_icloud_with_plus_strip() {
    assert_eq!(
        Email::from("foo+x@me.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@ICLOUD.COM"),
    );
}

#[test]
fn mac_dot_com_maps_to_icloud() {
    assert_eq!(
        Email::from("foo@mac.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@ICLOUD.COM"),
    );
}

#[test]
fn icloud_keeps_dots_in_local() {
    // Apple doesn't strip dots like Gmail does.
    assert_eq!(
        Email::from("foo.bar@icloud.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO.BAR@ICLOUD.COM"),
    );
}

#[test]
fn hotmail_maps_to_outlook() {
    assert_eq!(
        Email::from("foo@hotmail.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@OUTLOOK.COM"),
    );
}

#[test]
fn live_maps_to_outlook() {
    assert_eq!(
        Email::from("foo@live.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@OUTLOOK.COM"),
    );
}

#[test]
fn msn_maps_to_outlook() {
    assert_eq!(
        Email::from("foo@msn.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@OUTLOOK.COM"),
    );
}

#[test]
fn hotmail_co_uk_maps_to_outlook_and_strips_plus() {
    assert_eq!(
        Email::from("foo+x@hotmail.co.uk")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO@OUTLOOK.COM"),
    );
}

#[test]
fn outlook_keeps_dash_in_local() {
    assert_eq!(
        Email::from("foo-bar@outlook.com")
            .normalize()
            .as_ref()
            .map(|n| n.as_str()),
        Some("FOO-BAR@OUTLOOK.COM"),
    );
}

#[test]
fn normalizing_a_normalized_value_is_a_noop() {
    let original = Email::from("Steve+x@gmail.com");
    let once = original.normalize().expect("first normalize");
    let twice = Email::from(once.as_str())
        .normalize()
        .expect("second normalize");
    assert_eq!(once.as_str(), twice.as_str(), "normalize is idempotent");
}

#[test]
fn idempotency_holds_for_each_provider() {
    let cases = [
        "Foo@randomdomain.io",
        "foo-bar@Yahoo.CO.UK",
        "FOO@HOTMAIL.COM",
        "user@me.com",
        "S.T.E.V.E+x@GoogleMail.com",
    ];
    for original in cases {
        let once = Email::from(original).normalize().expect("first normalize");
        let twice = Email::from(once.as_str())
            .normalize()
            .expect("second normalize");
        assert_eq!(
            once.as_str(),
            twice.as_str(),
            "idempotent for input {original:?}"
        );
    }
}

#[test]
fn whitespace_only_input_rejected() {
    assert!(Email::from("   ").normalize().is_none());
}

#[test]
fn empty_input_rejected() {
    assert!(Email::from("").normalize().is_none());
}

// ── New T5 tests ──────────────────────────────────────────────────────────────

#[test]
fn email_normalize_round_trips_via_newtype() {
    let raw = Email::from("Steve+work@Gmail.com");
    let n = raw.normalize().unwrap();
    // normalize_str lowercases, strips plus (gmail rule), resolves alias →
    // then uppercases → "STEVE@GMAIL.COM"
    assert_eq!(n.as_str(), "STEVE@GMAIL.COM");
}

#[test]
fn normalized_email_serde_is_transparent() {
    let raw = Email::from("STEVE@GMAIL.COM");
    let n = raw.normalize().unwrap();
    let json = serde_json::to_string(&n).unwrap();
    assert_eq!(json, r#""STEVE@GMAIL.COM""#);
}

// ── NormalizerConfig builder ──────────────────────────────────────────────────

use super::{NormalizerConfig, OutputCase};

#[test]
fn builder_round_trip_preserves_options() {
    let cfg: NormalizerConfig = NormalizerConfig::builder()
        .output_case(OutputCase::Lowercase)
        .apply_provider_rules(false)
        .resolve_domain_aliases(false)
        .use_built_in_rules(false)
        .build();

    // Behavioral assertions live in the wiring tests; here we just prove
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

// ── normalize_with: toggle options ────────────────────────────────────────────

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
    // `f.o.o` proves the Gmail rule's local transform still runs (dots
    // are stripped) while the alias-resolution step is suppressed
    // (domain kept as `googlemail.com`).
    assert_eq!(
        Email::from("f.o.o@googlemail.com")
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

// ── Custom rules: stack + replace ─────────────────────────────────────────────

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
    // Gmail no longer gets its built-in dot-strip treatment: nothing
    // matches, DEFAULT_RULE applies (no dot-strip, just plus-strip).
    // Dots survive because the gmail built-in is bypassed; if it
    // weren't, this would be `FOO@GMAIL.COM`.
    assert_eq!(
        Email::from("f.o.o@gmail.com")
            .normalize_with(&cfg)
            .as_ref()
            .map(|n| n.as_str()),
        Some("F.O.O@GMAIL.COM"),
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
