use semver::Version;

/// Parses a version string leniently, accepting `MAJOR`, `MAJOR.MINOR`, or full
/// semver by zero-padding missing components. Strips a leading `v` prefix.
pub(super) fn parse_version_lenient(s: &str) -> Option<Version> {
    let s = s.strip_prefix('v').unwrap_or(s);
    if let Ok(v) = Version::parse(s) {
        return Some(v);
    }
    // Strip leading zeros from each component (semver rejects "22.04" due to "04").
    let normalize = |p: &str| -> String {
        p.parse::<u64>()
            .map_or_else(|_| p.to_string(), |n| n.to_string())
    };
    let parts: Vec<&str> = s.splitn(3, '.').collect();
    let padded = match parts.len() {
        1 => format!("{}.0.0", normalize(parts[0])),
        2 => format!("{}.{}.0", normalize(parts[0]), normalize(parts[1])),
        _ => {
            // Three parts present but standard parse failed; normalize each component.
            format!(
                "{}.{}.{}",
                normalize(parts[0]),
                normalize(parts[1]),
                normalize(parts[2])
            )
        }
    };
    Version::parse(&padded).ok()
}

/// Returns `Some(true)` when `actual >= required` (full semver), `Some(false)` when it is older,
/// and `None` when either string cannot be parsed.
pub(super) fn version_meets_minimum(required: &str, actual: &str) -> Option<bool> {
    let req = parse_version_lenient(required)?;
    let act = parse_version_lenient(actual)?;
    Some(act >= req)
}

/// Returns `Some(true)` when `actual.major >= required.major`, ignoring minor and patch.
/// Used for OS and kernel version checks where only the major release matters.
/// Returns `None` when either string cannot be parsed.
pub(super) fn major_version_meets_minimum(required: &str, actual: &str) -> Option<bool> {
    let req = parse_version_lenient(required)?;
    let act = parse_version_lenient(actual)?;
    Some(act.major >= req.major)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn version_meets_minimum_comparisons() {
        assert_eq!(version_meets_minimum("1.6.0", "1.6.0"), Some(true));
        assert_eq!(version_meets_minimum("1.6.0", "1.7.0"), Some(true));
        assert_eq!(version_meets_minimum("1.6.0", "1.5.9"), Some(false));
        assert_eq!(version_meets_minimum("11", "11.0.0"), Some(true));
        assert_eq!(version_meets_minimum("14.5", "14.4.1"), Some(false));
        assert_eq!(version_meets_minimum("14.5", "14.5.0"), Some(true));
    }

    #[test]
    fn major_version_meets_minimum_comparisons() {
        // Same major — always pass regardless of minor/patch.
        assert_eq!(major_version_meets_minimum("22.10", "22.04"), Some(true));
        assert_eq!(major_version_meets_minimum("22", "22.99.1"), Some(true));
        // Higher major — pass.
        assert_eq!(major_version_meets_minimum("5", "6.0.0"), Some(true));
        // Lower major — fail.
        assert_eq!(major_version_meets_minimum("6", "5.15.0"), Some(false));
        assert_eq!(major_version_meets_minimum("23", "22.04"), Some(false));
        // Unparseable — None.
        assert_eq!(major_version_meets_minimum("bad", "22.04"), None);
    }

    #[test]
    fn parse_version_lenient_handles_short_forms() {
        assert!(parse_version_lenient("11").is_some());
        assert!(parse_version_lenient("14.5").is_some());
        assert!(parse_version_lenient("1.6.0").is_some());
        assert!(parse_version_lenient("v1.6.0").is_some());
        assert!(parse_version_lenient("not-a-version").is_none());
    }
}
