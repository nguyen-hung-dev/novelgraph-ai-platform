pub const APP_NAME: &str = "NovelGraph AI Platform";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const API_VERSION: &str = "v0";
pub const RELEASE_CHANNEL: &str = "foundation";
pub const STORAGE_SCHEMA_VERSION: &str = "2026-04-27.foundation.v5";

#[cfg(test)]
mod tests {
    use super::{API_VERSION, APP_VERSION, RELEASE_CHANNEL, STORAGE_SCHEMA_VERSION};

    #[test]
    fn version_constants_are_populated() {
        assert!(!APP_VERSION.is_empty());
        assert!(API_VERSION.starts_with('v'));
        assert!(!RELEASE_CHANNEL.is_empty());
        assert!(!STORAGE_SCHEMA_VERSION.is_empty());
    }
}
