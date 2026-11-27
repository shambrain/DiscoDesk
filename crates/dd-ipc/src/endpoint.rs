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
/// * Windows: `\\.\pipe\discodesk-engine-<user>`
/// * Unix: `$XDG_RUNTIME_DIR/discodesk-engine-<user>.sock`
#[must_use]
pub fn default_endpoint_path() -> String {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "default".to_owned());

    let segment = sanitise_user_segment(&user);

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
    fn user_segment_is_bounded() {
        let long = "x".repeat(200);
        assert_eq!(sanitise_user_segment(&long).len(), MAX_USER_SEGMENT);
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
}
