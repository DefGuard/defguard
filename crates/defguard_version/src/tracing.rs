use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use tracing_subscriber::{
    fmt::{FormatEvent, FormatFields, format::Writer},
    {layer::SubscriberExt, util::SubscriberInitExt},
};

use crate::{ComponentInfo, DefguardVersionSet};

/// Custom tracing formatter that conditionally includes version information in log messages.
///
/// This formatter wraps the default tracing formatter and adds version prefixes to log messages
/// under specific conditions:
/// - Always includes version info for ERROR level logs
/// - Always includes version info for logs within specified span names (regardless of log level)
///
/// The version information includes details about the application and connected services,
/// along with comprehensive system information for debugging purposes.
struct VersionPrefixFormat {
    /// The underlying tracing formatter
    inner: tracing_subscriber::fmt::format::Format,
    /// Shared version information of all components
    version_set: Arc<DefguardVersionSet>,
    /// Set of span names that should always include version info in their logs
    always_version_spans: HashSet<String>,
}

impl<S, N> FormatEvent<S, N> for VersionPrefixFormat
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    /// Formats a tracing event, conditionally adding version information as a prefix.
    ///
    /// This method checks if version information should be included based on:
    /// 1. Log level (always include for ERROR level)
    /// 2. Span context (include if any span in the current context matches configured span names)
    ///
    /// The version prefix format includes comprehensive system information:
    /// `[v{version}|{os_type}|{os_version}|{bitness}|{architecture}]`
    ///
    /// Additional prefixes are added for connected services:
    /// - Core service: `[C:v{version}|...]`
    /// - Proxy service: `[PX:v{version}|...]`
    /// - Gateway service: `[GW:v{version}|...]`
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let is_error_level = *event.metadata().level() == tracing::Level::ERROR;

        // Check if we're within any of the configured spans
        let is_in_version_span = ctx.lookup_current().map_or(false, |span_ref| {
            let mut current_span = Some(span_ref);
            while let Some(span) = current_span {
                let span_name = span.metadata().name();
                if self.always_version_spans.contains(span_name) {
                    return true;
                }
                current_span = span.parent();
            }
            false
        });

        let should_log_version = is_error_level || is_in_version_span;
        if should_log_version {
            write!(
                writer,
                "[v{}|{}|{}|{}|{}] ",
                self.version_set.own.version,
                self.version_set.own.system.os_type,
                self.version_set.own.system.os_version,
                self.version_set.own.system.bitness,
                self.version_set.own.system.architecture,
            )?;

            if let Some(ref core) = *self.version_set.core.read().unwrap() {
                write!(
                    writer,
                    "[C:v{}|{}|{}|{}|{}] ",
                    core.version,
                    core.system.os_type,
                    core.system.os_version,
                    core.system.bitness,
                    core.system.architecture,
                )?;
            }
            if let Some(ref proxy) = *self.version_set.proxy.read().unwrap() {
                write!(
                    writer,
                    "[PX:v{}|{}|{}|{}|{}] ",
                    proxy.version,
                    proxy.system.os_type,
                    proxy.system.os_version,
                    proxy.system.bitness,
                    proxy.system.architecture,
                )?;
            }

            if let Some(ref gateway) = *self.version_set.gateway.read().unwrap() {
                write!(
                    writer,
                    "[GW:v{}|{}|{}|{}|{}] ",
                    gateway.version,
                    gateway.system.os_type,
                    gateway.system.os_version,
                    gateway.system.bitness,
                    gateway.system.architecture,
                )?;
            }
        }

        self.inner.format_event(ctx, writer, event)
    }
}

/// Initializes tracing with custom formatter displaying own and connected services version.
/// Returns shared VersionInfo object that will be used to display services versions.
///
/// # Arguments
/// * `version` - The application version
/// * `log_level` - The log level to use
/// * `always_version_spans` - Span names that should always include version info in logs
///
/// # Examples
/// ```
/// let version_set = defguard_version::init_tracing(
///     "1.5.0",
///     "info",
///     &["run_grpc_bidi_stream", "enrollment_process"]
/// );
/// ```
pub fn init(
    version: &str,
    log_level: &str,
    always_version_spans: &[&str],
) -> Arc<DefguardVersionSet> {
    let version_set = Arc::new(DefguardVersionSet {
        own: ComponentInfo::try_from(version).expect("Failed to parse version: {version}"),
        core: Arc::new(RwLock::new(None)),
        proxy: Arc::new(RwLock::new(None)),
        gateway: Arc::new(RwLock::new(None)),
    });

    let spans: HashSet<String> = always_version_spans
        .iter()
        .map(|&s| s.to_string())
        .collect();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{},h2=info", log_level).into()),
        )
        .with(
            tracing_subscriber::fmt::layer().event_format(VersionPrefixFormat {
                inner: tracing_subscriber::fmt::format::Format::default(),
                version_set: Arc::clone(&version_set),
                always_version_spans: spans,
            }),
        )
        .init();
    version_set
}
