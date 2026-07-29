const VERSION: &str = env!("CARGO_PKG_VERSION");
const REVISION: &str = env!("OXIDEFALL_GIT_SHA");

pub(crate) fn display_label() -> String {
    format_display(VERSION, revision(), dirty())
}

pub(crate) fn diagnostic_report() -> String {
    format_report(VERSION, revision(), dirty(), target(), profile())
}

fn revision() -> Option<&'static str> {
    (!REVISION.is_empty()).then_some(REVISION)
}

fn dirty() -> bool {
    env!("OXIDEFALL_GIT_DIRTY") == "true"
}

const fn target() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "web"
    } else {
        "native"
    }
}

const fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn format_display(version: &str, revision: Option<&str>, dirty: bool) -> String {
    let mut label = format!("v{version}");
    if let Some(revision) = revision {
        label.push_str(" · ");
        label.push_str(&revision[..revision.len().min(7)]);
        if dirty {
            label.push_str("+dirty");
        }
    }
    label
}

fn format_report(
    version: &str,
    revision: Option<&str>,
    dirty: bool,
    target: &str,
    profile: &str,
) -> String {
    let mut report = format!("Oxidefall v{version}");
    if let Some(revision) = revision {
        report.push_str(" · ");
        report.push_str(revision);
        if dirty {
            report.push_str("+dirty");
        }
    }
    report.push_str(" · ");
    report.push_str(target);
    report.push_str(" · ");
    report.push_str(profile);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "d0585a57ee4f699fa591b67d5eef61afc83dc08d";

    #[test]
    fn display_uses_short_revision_and_dirty_marker() {
        assert_eq!(
            format_display("0.1.0", Some(SHA), true),
            "v0.1.0 · d0585a5+dirty"
        );
    }

    #[test]
    fn report_uses_full_revision_target_and_profile() {
        assert_eq!(
            format_report("0.1.0", Some(SHA), false, "web", "release"),
            format!("Oxidefall v0.1.0 · {SHA} · web · release")
        );
    }

    #[test]
    fn missing_revision_falls_back_to_package_version() {
        assert_eq!(format_display("0.1.0", None, false), "v0.1.0");
        assert_eq!(
            format_report("0.1.0", None, false, "native", "debug"),
            "Oxidefall v0.1.0 · native · debug"
        );
    }
}
