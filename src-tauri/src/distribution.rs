/// Compatibility builds use manual releases so upstream updates cannot remove the patch.
pub(crate) fn is_aigocode_compat() -> bool {
    option_env!("AIGOCODE_COMPAT_BUILD") == Some("1")
}

pub(crate) fn releases_url() -> &'static str {
    if is_aigocode_compat() {
        option_env!("AIGOCODE_RELEASES_URL")
            .filter(|url| !url.is_empty())
            .unwrap_or("https://github.com/jupiterchu/cc-switch/releases")
    } else {
        "https://github.com/farion1231/cc-switch/releases/latest"
    }
}
