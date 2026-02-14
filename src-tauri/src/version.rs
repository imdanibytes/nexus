use std::cmp::Ordering;

/// Compare two semver version strings.
/// Returns `None` if either string fails to parse.
pub fn compare_versions(installed: &str, available: &str) -> Option<Ordering> {
    let installed = semver::Version::parse(installed).ok()?;
    let available = semver::Version::parse(available).ok()?;
    Some(installed.cmp(&available))
}

/// Check if `current` is greater than or equal to `min`.
/// Returns `false` if either string fails to parse.
#[allow(dead_code)]
pub fn satisfies_min_version(current: &str, min: &str) -> bool {
    compare_versions(current, min)
        .map(|ord| ord != Ordering::Less)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_version_detected() {
        assert_eq!(
            compare_versions("1.0.0", "2.0.0"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn same_version() {
        assert_eq!(
            compare_versions("1.2.3", "1.2.3"),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn older_version() {
        assert_eq!(
            compare_versions("3.0.0", "2.0.0"),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn invalid_version_returns_none() {
        assert_eq!(compare_versions("not-a-version", "1.0.0"), None);
    }

    #[test]
    fn satisfies_min_when_equal() {
        assert!(satisfies_min_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn satisfies_min_when_greater() {
        assert!(satisfies_min_version("2.0.0", "1.0.0"));
    }

    #[test]
    fn does_not_satisfy_min_when_less() {
        assert!(!satisfies_min_version("0.9.0", "1.0.0"));
    }

    #[test]
    fn does_not_satisfy_on_invalid() {
        assert!(!satisfies_min_version("garbage", "1.0.0"));
    }

    #[test]
    fn prerelease_less_than_release() {
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn prerelease_ordering() {
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0-beta"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn missing_patch_returns_none() {
        // semver requires major.minor.patch — "1.0" is invalid
        assert_eq!(compare_versions("1.0", "1.0.0"), None);
    }

    #[test]
    fn both_invalid_returns_none() {
        assert_eq!(compare_versions("abc", "xyz"), None);
    }

    #[test]
    fn patch_bump_detected() {
        assert_eq!(
            compare_versions("1.0.0", "1.0.1"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn build_metadata_ordering() {
        // The semver crate compares build metadata for total ordering.
        // Same build = Equal; different builds have a defined order.
        assert_eq!(
            compare_versions("1.0.0+build1", "1.0.0+build1"),
            Some(Ordering::Equal)
        );
        // Different builds are ordered but both > and < are valid — just verify it parses
        assert!(compare_versions("1.0.0+build1", "1.0.0+build2").is_some());
    }
}
