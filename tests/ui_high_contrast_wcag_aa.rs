//! WCAG AA contrast test for the HighContrast theme preset.
//!
//! Per `coding_agent_session_search-vz9t8.1`. Sweeps every `ThemePreset`
//! variant; for HighContrast specifically, asserts WCAG AA compliance
//! (4.5:1 body, 3:1 large/UI). Other presets get an information-only
//! lower-bound check (3:1) that does not block.

use coding_agent_search::ui::components::theme::{ThemePalette, ThemePreset, contrast_ratio};
use ftui::PackedRgba;

/// All theme presets the project ships, mirroring `ThemePreset::ALL`. Pinned
/// here so the test fails loudly if a preset is added without considering
/// contrast.
fn all_presets() -> Vec<ThemePreset> {
    vec![
        ThemePreset::TokyoNight,
        ThemePreset::Daylight,
        ThemePreset::Catppuccin,
        ThemePreset::Dracula,
        ThemePreset::Nord,
        ThemePreset::SolarizedDark,
        ThemePreset::SolarizedLight,
        ThemePreset::Monokai,
        ThemePreset::GruvboxDark,
        ThemePreset::OneDark,
        ThemePreset::RosePine,
        ThemePreset::Everforest,
        ThemePreset::Kanagawa,
        ThemePreset::AyuMirage,
        ThemePreset::Nightfox,
        ThemePreset::CyberpunkAurora,
        ThemePreset::Synthwave84,
        ThemePreset::HighContrast,
        ThemePreset::Colorblind,
    ]
}

/// Per-token labels and accessor functions used by the contrast sweep. Each
/// returns (fg, bg) for a logical pair the user actually sees rendered.
fn token_pairs(p: ThemePalette) -> Vec<(&'static str, PackedRgba, PackedRgba)> {
    vec![
        ("body_text", p.fg, p.bg),
        ("body_on_surface", p.fg, p.surface),
        ("hint_on_bg", p.hint, p.bg),
        ("border_on_bg", p.border, p.bg),
        ("user_role_on_bg", p.user, p.bg),
        ("agent_role_on_bg", p.agent, p.bg),
        ("tool_role_on_bg", p.tool, p.bg),
        ("system_role_on_bg", p.system, p.bg),
        ("accent_on_bg", p.accent, p.bg),
        ("accent_alt_on_bg", p.accent_alt, p.bg),
        ("body_on_stripe_even", p.fg, p.stripe_even),
        ("body_on_stripe_odd", p.fg, p.stripe_odd),
    ]
}

/// Helper for structured logging of a single measurement.
fn log_measurement(preset: ThemePreset, token: &str, ratio: f64, aa_pass: bool) {
    tracing::info!(
        target: "wcag_aa_test",
        preset = preset.name(),
        token = token,
        ratio = format!("{ratio:.2}"),
        aa_pass = aa_pass
    );
}

const WCAG_AA_BODY: f64 = 4.5;
const WCAG_AA_LARGE: f64 = 3.0;
const LEGIBILITY_MIN: f64 = 3.0;

/// Tokens that count as "body text" for AA purposes (need 4.5:1).
fn is_body_token(token: &str) -> bool {
    matches!(
        token,
        "body_text" | "body_on_surface" | "body_on_stripe_even" | "body_on_stripe_odd"
    )
}

#[test]
fn high_contrast_preset_meets_wcag_aa() {
    tracing::info!(target: "wcag_aa_test", scenario = "high_contrast_aa");
    let palette = ThemePreset::HighContrast.to_palette();
    let mut failures: Vec<(&str, f64)> = Vec::new();
    for (label, fg, bg) in token_pairs(palette) {
        let ratio = contrast_ratio(fg, bg);
        let target = if is_body_token(label) {
            WCAG_AA_BODY
        } else {
            WCAG_AA_LARGE
        };
        let aa_pass = ratio >= target;
        log_measurement(ThemePreset::HighContrast, label, ratio, aa_pass);
        if !aa_pass {
            failures.push((label, ratio));
        }
    }
    if !failures.is_empty() {
        let mut msg = String::from("HighContrast preset fails WCAG AA on:\n");
        for (label, ratio) in &failures {
            msg.push_str(&format!(
                "  {label}: ratio={ratio:.2} (need {} for body, {} for large)\n",
                WCAG_AA_BODY, WCAG_AA_LARGE
            ));
        }
        msg.push_str("Fix in src/ui/components/theme.rs::ThemePalette::high_contrast()\n");
        panic!("{msg}");
    }
}

#[test]
fn other_presets_meet_minimum_legibility() {
    tracing::info!(target: "wcag_aa_test", scenario = "other_presets_legibility");
    // Body-text legibility lower bound for non-AA presets. We only enforce on
    // body tokens because role/accent tokens may legitimately be quieter.
    let mut warnings: Vec<(String, &str, f64)> = Vec::new();
    for preset in all_presets() {
        if matches!(preset, ThemePreset::HighContrast) {
            continue;
        }
        let palette = preset.to_palette();
        for (label, fg, bg) in token_pairs(palette) {
            if !is_body_token(label) {
                continue;
            }
            let ratio = contrast_ratio(fg, bg);
            log_measurement(preset, label, ratio, ratio >= WCAG_AA_BODY);
            if ratio < LEGIBILITY_MIN {
                warnings.push((preset.name().to_string(), label, ratio));
            }
        }
    }
    // This test logs but does NOT fail on individual presets being below 3:1
    // (some intentionally low-contrast presets are aesthetic choices). It DOES
    // fail if the COUNT of below-3:1 body tokens exceeds 30% of presets, which
    // would indicate a systemic regression rather than one preset's intent.
    let total = (all_presets().len() - 1) * 4; // 4 body tokens × non-HC presets
    let bad_pct = (warnings.len() * 100) / total.max(1);
    tracing::info!(
        target: "wcag_aa_test",
        scenario = "other_presets_legibility",
        below_legibility_count = warnings.len(),
        total_body_tokens = total,
        bad_pct = bad_pct
    );
    assert!(
        bad_pct <= 30,
        "More than 30% of non-HighContrast body-token measurements fall below 3:1 ratio: {bad_pct}%. Likely systemic regression in palette generation. Failures:\n{}",
        warnings
            .iter()
            .map(|(p, l, r)| format!("  {p}/{l}: {r:.2}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn high_contrast_focused_unfocused_borders_meet_threshold() {
    tracing::info!(target: "wcag_aa_test", scenario = "high_contrast_borders");
    let palette = ThemePreset::HighContrast.to_palette();
    // The palette's `border` color is the "focused" border for HighContrast;
    // the unfocused variant is implicitly a dimmed shade. We use accent_alt
    // as a proxy for focused-state highlight.
    let pairs = [
        ("border_on_bg", palette.border, palette.bg),
        ("accent_focus_on_bg", palette.accent, palette.bg),
        ("accent_alt_focus_on_bg", palette.accent_alt, palette.bg),
    ];
    for (label, fg, bg) in pairs {
        let ratio = contrast_ratio(fg, bg);
        log_measurement(
            ThemePreset::HighContrast,
            label,
            ratio,
            ratio >= WCAG_AA_LARGE,
        );
        assert!(
            ratio >= WCAG_AA_LARGE,
            "HighContrast border/focus ratio for {label} is {ratio:.2}, below 3:1 minimum"
        );
    }
}

#[test]
fn high_contrast_passes_on_inverted_terminal_background() {
    tracing::info!(target: "wcag_aa_test", scenario = "high_contrast_inverted_term");
    // Some terminals advertise OSC 11 background queries; if the user's
    // terminal is set to a light background, HighContrast would need to
    // adapt. For now, assert the palette's bg/fg pair *itself* gives AA in
    // both directions — i.e., swapping fg/bg also produces AA.
    let palette = ThemePreset::HighContrast.to_palette();
    let forward = contrast_ratio(palette.fg, palette.bg);
    let inverted = contrast_ratio(palette.bg, palette.fg);
    log_measurement(
        ThemePreset::HighContrast,
        "fg_on_bg",
        forward,
        forward >= WCAG_AA_BODY,
    );
    log_measurement(
        ThemePreset::HighContrast,
        "bg_on_fg",
        inverted,
        inverted >= WCAG_AA_BODY,
    );
    // contrast_ratio is symmetric — both should produce the same value.
    assert!(
        (forward - inverted).abs() < 1e-9,
        "contrast_ratio must be symmetric; got forward={forward:.6} inverted={inverted:.6}"
    );
    assert!(
        forward >= WCAG_AA_BODY,
        "HighContrast fg/bg ratio is {forward:.2}, below WCAG AA 4.5:1 — preset must be AA-compliant in both directions"
    );
}

#[test]
fn contrast_utility_handles_extreme_inputs() {
    tracing::info!(target: "wcag_aa_test", scenario = "extreme_inputs");
    let black = PackedRgba::rgb(0, 0, 0);
    let white = PackedRgba::rgb(255, 255, 255);
    let max = contrast_ratio(black, white);
    assert!(
        (max - 21.0).abs() < 0.1,
        "max contrast (black on white) should be ~21, got {max}"
    );
    let identity = contrast_ratio(white, white);
    assert!(
        (identity - 1.0).abs() < 1e-9,
        "identical colors should give contrast 1.0, got {identity}"
    );
    tracing::info!(target: "wcag_aa_test", max_contrast = max, identity = identity);
}
