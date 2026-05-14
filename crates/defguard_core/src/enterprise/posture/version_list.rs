/// Minimum defguard desktop client versions available for posture rules.
/// FIXME: 2.0 does not actually exist, remove before release
pub const CLIENT_VERSIONS: &[&str] = &["1.6", "2.0"];

pub const WINDOWS_OS_VERSIONS: &[i32] = &[10, 11];
pub const MACOS_OS_VERSIONS: &[i32] = &[13, 14, 15, 26];
pub const IOS_OS_VERSIONS: &[i32] = &[17, 18, 26];
pub const ANDROID_OS_VERSIONS: &[i32] = &[13, 14, 15, 16];

/// Valid Linux kernel major versions for posture rules.
pub const LINUX_KERNEL_VERSIONS: &[i32] = &[5, 6, 7];
