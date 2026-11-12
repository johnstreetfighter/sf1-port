//! Workspace release identity shared by the front end, tools and protocols.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_SHA: &str = env!("SF1_GIT_SHA");
pub const EDITION_NAME: &str = edition_name(VERSION);

/// The edition follows the major release; feature projects advance its minor.
/// Unrecognized versions use the original title until their edition is defined.
pub const fn edition_name(version: &str) -> &'static str {
    if has_prefix(version, "1.0.") {
        "Street Fighter: The World Tournament"
    } else if has_prefix(version, "1.") {
        "Street Fighter': The World Tournament"
    } else if has_prefix(version, "2.") {
        "Street Fighter: The World Tournament — Anniversary Edition"
    } else if has_prefix(version, "3.") {
        "Super Street Fighter: Rising"
    } else if has_prefix(version, "4.") {
        "Hyper Street Fighter: Bloodlines"
    } else if has_prefix(version, "5.") {
        "Ultra Street Fighter: Final Round"
    } else {
        "Street Fighter: The World Tournament"
    }
}

const fn has_prefix(value: &str, prefix: &str) -> bool {
    let value = value.as_bytes();
    let prefix = prefix.as_bytes();
    if value.len() < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if value[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Compact ASCII label for the Settings header's small bitmap font.
pub const fn short_stamp() -> &'static str {
    concat!("V", env!("CARGO_PKG_VERSION"))
}

pub fn full_banner() -> String {
    format!("{EDITION_NAME} v{VERSION} ({GIT_SHA})")
}

/// Netplay compatibility includes the release, source revision, and the
/// synchronized rules layout. Keep the packet envelope readable by older peers.
pub const fn protocol_version_hash() -> u32 {
    const STYLE_RULES: u32 = 8;
    (identity_hash(VERSION, GIT_SHA) ^ STYLE_RULES).wrapping_mul(0x0100_0193)
}

const fn identity_hash(version: &str, sha: &str) -> u32 {
    let mut hash = 0x811c_9dc5_u32;
    let mut i = 0;
    while i < version.len() {
        hash = (hash ^ version.as_bytes()[i] as u32).wrapping_mul(0x0100_0193);
        i += 1;
    }
    i = 0;
    while i < sha.len() {
        hash = (hash ^ sha.as_bytes()[i] as u32).wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edition_lineage_survives_patch_and_feature_releases() {
        for (versions, name) in [
            (["1.0.0", "1.0.12"], "Street Fighter: The World Tournament"),
            (["1.1.0", "1.10.3"], "Street Fighter': The World Tournament"),
            (
                ["2.0.0", "2.4.1"],
                "Street Fighter: The World Tournament — Anniversary Edition",
            ),
            (["3.0.0", "3.2.1"], "Super Street Fighter: Rising"),
            (["4.0.0", "4.3.2"], "Hyper Street Fighter: Bloodlines"),
            (["5.0.0", "5.1.9"], "Ultra Street Fighter: Final Round"),
        ] {
            for version in versions {
                assert_eq!(edition_name(version), name, "{version}");
            }
        }
        for unknown in ["", "0.1.0", "10.0.0"] {
            assert_eq!(
                edition_name(unknown),
                "Street Fighter: The World Tournament"
            );
        }
    }

    #[test]
    fn labels_share_the_workspace_identity() {
        assert_eq!(short_stamp(), format!("V{VERSION}"));
        assert_eq!(EDITION_NAME, edition_name(VERSION));
        assert_eq!(
            full_banner(),
            format!("{EDITION_NAME} v{VERSION} ({GIT_SHA})")
        );
        assert!(!GIT_SHA.is_empty());
    }

    #[test]
    fn compatibility_hash_uses_version_and_revision() {
        // Standard FNV-1a test vector, split across the two identity fields.
        assert_eq!(identity_hash("foo", "bar"), 0xbf9c_f968);
        assert_ne!(
            identity_hash("1.1.0", "12345678"),
            identity_hash("1.2.0", "12345678")
        );
        assert_ne!(
            identity_hash("1.1.0", "12345678"),
            identity_hash("1.1.0", "87654321")
        );
        const HASH: u32 = protocol_version_hash();
        assert_ne!(HASH, identity_hash(VERSION, GIT_SHA));
        assert_eq!(HASH, protocol_version_hash());
    }
}
