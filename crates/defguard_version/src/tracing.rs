use tracing::{Level, Subscriber};
use tracing_subscriber::{
    Layer,
    fmt::{FormatEvent, FormatFields, format::Writer},
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
};

use crate::SystemInfo;

/// Custom tracing formatter that conditionally includes version information in log messages.
///
/// This formatter wraps the default tracing formatter and adds version prefixes to log messages:
/// - For ERROR level logs: includes core_version, proxy_version, and proxy_info (if available)
/// - For other levels: includes only core_version and proxy_version (if available)
///
/// The version information is extracted from tracing span fields.
struct VersionPrefixFormat {
    /// The underlying tracing formatter
    inner: tracing_subscriber::fmt::format::Format,
    /// The core application version to display as fallback
    own_version: String,
    own_info: SystemInfo,
}

/// A layer that captures version fields from spans and stores them for use by the formatter
struct VersionFieldLayer;

impl<S> Layer<S> for VersionFieldLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = SpanFieldVisitor::default();
            attrs.record(&mut visitor);
            span.extensions_mut().insert(visitor);
        }
    }
}

impl<S, N> FormatEvent<S, N> for VersionPrefixFormat
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    /// Formats a tracing event, conditionally adding version information as a prefix.
    ///
    /// This method includes version information based on:
    /// - For ERROR level logs: includes core_version, proxy_version, and proxy_info (if available in span)
    /// - For other levels: includes only core_version and proxy_version (if available in span)
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        // Extract version information from current span context
        let mut proxy_version = None;
        let mut proxy_info = None;

        if let Some(span_ref) = ctx.lookup_current() {
            let mut current_span = Some(span_ref);
            while let Some(span) = current_span {
                let extensions = span.extensions();

                // Try to get stored visitor from span extensions
                if let Some(stored_visitor) = extensions.get::<SpanFieldVisitor>() {
                    if proxy_version.is_none() && stored_visitor.proxy_version.is_some() {
                        proxy_version = stored_visitor.proxy_version.clone();
                    }
                    if proxy_info.is_none() && stored_visitor.proxy_info.is_some() {
                        proxy_info = stored_visitor.proxy_info.clone();
                    }
                }

                current_span = span.parent();
            }
        }

        // Format version prefix based on log level and available information
        let is_versioned_span = proxy_version.is_some();
        let is_error = *event.metadata().level() == Level::ERROR;
        if is_versioned_span || is_error {
            // Own version
            let mut own_version_str = format!("[{}", self.own_version);
            if is_error {
                own_version_str = format!("{own_version_str} {}", self.own_info);
            }
            own_version_str = format!("{own_version_str}]");
            write!(writer, "{}", own_version_str)?;
        }
        // Proxy version
        if let Some(ref proxy_version) = proxy_version {
            let mut proxy_version_str = format!("[PX:{}", proxy_version);
            if is_error {
                if let Some(ref proxy_info) = proxy_info {
                    proxy_version_str = format!("{proxy_version_str} {}", proxy_info);
                }
            }
            proxy_version_str = format!("{proxy_version_str}]");
            write!(writer, "{}", proxy_version_str)?;
        }

        self.inner.format_event(ctx, writer, event)
    }
}

/// A visitor that extracts version fields from spans
#[derive(Default, Clone)]
struct SpanFieldVisitor {
    proxy_version: Option<String>,
    proxy_info: Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "proxy_version" => self.proxy_version = Some(value.to_string()),
            "proxy_info" => self.proxy_info = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "proxy_version" => self.proxy_version = Some(format!("{:?}", value)),
            "proxy_info" => self.proxy_info = Some(format!("{:?}", value)),
            _ => {}
        }
    }
}

/// Initializes tracing with custom formatter that conditionally displays version information.
///
/// The formatter will:
/// - For ERROR level logs: display core_version, proxy_version, and proxy_info (if available)
/// - For other log levels: display only core_version and proxy_version (if available)
///
/// Version information is extracted from tracing span fields with names:
/// - `core_version`: The core application version
/// - `proxy_version`: The connected proxy version
/// - `proxy_info`: Additional proxy system information
///
/// # Arguments
/// * `core_version` - The core application version to use as fallback when not found in spans
/// * `log_level` - The log level filter to use
///
/// # Examples
/// ```
/// defguard_version::tracing::init("1.5.0", "info");
/// ```
pub fn init(own_version: &str, log_level: &str) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{},h2=info", log_level).into()),
        )
        .with(VersionFieldLayer) // Add our custom layer to capture span fields
        .with(
            tracing_subscriber::fmt::layer().event_format(VersionPrefixFormat {
                inner: tracing_subscriber::fmt::format::Format::default(),
                own_version: own_version.to_string(),
                own_info: SystemInfo::get(),
            }),
        )
        .init();
}
