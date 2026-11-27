//! Endpoint naming shared by the engine and the Studio.
//!
//! Both processes must compute the *same* default endpoint without talking to
//! each other first, otherwise they silently open two different pipes. That
//! makes this a shared contract, not an engine implementation detail, so it
//! lives in `dd-ipc` rather than in either binary.
//!
//! Security does **not** depend on the name: the Windows pipe carries a
//! current-user security descriptor regardless of what it is called. The name
//! only prevents two user accounts on one machine from colliding.
//!
//! This is intentionally *not* derived from the process id. A pid-derived name
//! would make the endpoint unguessable to the Studio, which is the whole point
//! of the name being shared.

/// The stable stem of every DiscoDesk engine endpoint.
pub const ENDPOINT_STEM: &str = "discodesk-engine";

/// The maximum length of the sanitised user segment.
const MAX_USER_SEGMENT: usize = 32;

/// The endpoint the engine listens on and the Studio connects to.
///
/// * Windows: `\\.\pipe\discodesk-engine-<segment>`
/// * Unix: `$XDG_RUNTIME_DIR/discodesk-engine-<segment>.sock`
#[must_use]
pub fn default_endpoint_path() -> String {
    let segment = user_segment();

    #[cfg(windows)]
    {
        format!(r"\\.\pipe\{ENDPOINT_STEM}-{segment}")
    }
    #[cfg(unix)]
    {
        let dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        format!("{dir}/{ENDPOINT_STEM}-{segment}.sock")
    }
    #[cfg(not(any(windows, unix)))]
    {
        format!("{ENDPOINT_STEM}-{segment}")
    }
}

/// A short, stable, per-account path segment.
///
/// On Windows this is derived from the **account SID**, not from an environment
/// variable. That matters: `USERNAME`/`USER` can be empty or overridden, and an
/// empty value silently collapses every account on the machine onto one name —
/// which was observed on this project's own development shell. A SID is the
/// authoritative local identity and cannot be spoofed by setting an env var.
///
/// The SID is hashed rather than embedded. A raw SID is up to ~45 characters and
/// truncating it would collide for accounts that share a machine prefix; a hash
/// is fixed-length and distinct.
///
/// This is not a security boundary (the pipe ACL is): it only prevents two
/// accounts from fighting over one endpoint name.
#[must_use]
fn user_segment() -> String {
    #[cfg(windows)]
    if let Ok(sid) = crate::transport::windows::current_user_sid()
        && !sid.is_empty()
    {
        return format!("u{:016x}", fnv1a64(sid.as_bytes()));
    }

    match env_user_name() {
        Some(name) => sanitise_user_segment(&name),
        None => "default".to_owned(),
    }
}

/// The OS-reported user name, ignoring empty or whitespace-only values.
fn env_user_name() -> Option<String> {
    ["USERNAME", "USER"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.trim().is_empty())
}

/// FNV-1a (64-bit).
///
/// Used only to shorten an identity into a stable path segment. It is not used
/// for anything security-sensitive; the pipe ACL provides the access control.
const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

/// Reduces a user name to a safe, bounded path segment.
///
/// Only ASCII alphanumerics survive; everything else (including `\`, `/`, `.`
/// and whitespace) becomes `-`. An empty result becomes `default` so the name
/// can never end in a bare separator.
#[must_use]
fn sanitise_user_segment(user: &str) -> String {
    let sanitised: String = user
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(MAX_USER_SEGMENT)
        .collect();

    if sanitised.is_empty() {
        "default".to_owned()
    } else {
        sanitised
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_endpoint_contains_the_stem() {
        let endpoint = default_endpoint_path();
        assert!(!endpoint.is_empty());
        assert!(endpoint.contains(ENDPOINT_STEM), "got {endpoint}");
    }

    #[test]
    fn default_endpoint_is_stable_across_calls() {
        // Two processes in the same user session must agree.
        assert_eq!(default_endpoint_path(), default_endpoint_path());
    }

    #[cfg(windows)]
    #[test]
    fn default_endpoint_is_a_local_pipe_path() {
        assert!(default_endpoint_path().starts_with(r"\\.\pipe\"));
    }

    #[test]
    fn user_segment_strips_path_separators_and_spaces() {
        assert_eq!(sanitise_user_segment(r"a\b/c d"), "a-b-c-d");
    }

    #[test]
    fn empty_user_segment_falls_back() {
        assert_eq!(sanitise_user_segment(""), "default");
    }

    #[test]
    fn non_ascii_is_replaced_not_dropped_silently() {
        // A non-ASCII name must still produce a usable, distinct-looking
        // segment rather than an empty one.
        let segment = sanitise_user_segment("caf\u{00e9}");
        assert!(segment.starts_with("caf-"), "got {segment}");
    }

    #[test]
    fn fnv1a_is_deterministic_and_distinguishes_identities() {
        assert_eq!(fnv1a64(b"S-1-5-21-a"), fnv1a64(b"S-1-5-21-a"));
        assert_ne!(fnv1a64(b"S-1-5-21-a"), fnv1a64(b"S-1-5-21-b"));
        // Known value for the empty input, which catches an accidental change
        // of algorithm or offset basis.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn user_segment_is_non_empty_and_path_safe() {
        let segment = user_segment();
        assert!(!segment.is_empty());
        assert!(
            segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "segment must be path safe, got {segment}"
        );
    }

    #[test]
    fn empty_environment_falls_back_rather_than_producing_a_bare_separator() {
        // Documents the reason the SID is preferred: an empty USERNAME must not
        // yield a segment-less endpoint. This exercises the fallback directly,
        // without mutating the process environment.
        assert_eq!(env_user_name_from(&[("", ""), ("USER", "  ")]), None);
        assert_eq!(
            env_user_name_from(&[("USERNAME", ""), ("USER", "alice")]).as_deref(),
            Some("alice")
        );
    }

    /// Mirrors [`env_user_name`] over an explicit set of pairs so the fallback
    /// can be tested without mutating process-global state.
    fn env_user_name_from(pairs: &[(&str, &str)]) -> Option<String> {
        ["USERNAME", "USER"]
            .iter()
            .filter_map(|key| {
                pairs
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| (*value).to_owned())
            })
            .find(|value| !value.trim().is_empty())
    }
}
