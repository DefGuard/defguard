use tracing::{Level, Subscriber};
use tracing_subscriber::{
    Layer,
    fmt::{FormatEvent, FormatFields, format::Writer, FmtContext},
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
    field::RecordFields,
    registry::LookupSpan,
};

use crate::SystemInfo;

/// Extracted version information from span context
#[derive(Debug, Default, Clone)]
pub struct ExtractedVersionInfo {
    pub core_version: Option<String>,
    pub core_info: Option<String>,
    pub proxy_version: Option<String>,
    pub proxy_info: Option<String>,
    pub gateway_version: Option<String>,
    pub gateway_info: Option<String>,
}

impl ExtractedVersionInfo {
    /// Check if any version information is present
    pub fn has_version_info(&self) -> bool {
        self.core_version.is_some() || self.proxy_version.is_some() || self.gateway_version.is_some()
    }
}

/// Extract version information from current span context
/// 
/// This function walks up the span hierarchy and extracts version information 
/// from span extensions that were stored by VersionFieldLayer.
/// 
/// # Arguments
/// * `ctx` - The format context from the tracing formatter
/// 
/// # Returns
/// An `ExtractedVersionInfo` struct containing all version information found in the span hierarchy
pub fn extract_version_info_from_context<S, N>(
    ctx: &FmtContext<'_, S, N>,
) -> ExtractedVersionInfo
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    let mut extracted = ExtractedVersionInfo::default();

    if let Some(span_ref) = ctx.lookup_current() {
        let mut current_span = Some(span_ref);
        while let Some(span) = current_span {
            let extensions = span.extensions();

            if let Some(stored_visitor) = extensions.get::<SpanFieldVisitor>() {
                if extracted.core_version.is_none() && stored_visitor.core_version.is_some() {
                    extracted.core_version = stored_visitor.core_version.clone();
                    extracted.core_info = stored_visitor.core_info.clone();
                }
                if extracted.proxy_version.is_none() && stored_visitor.proxy_version.is_some() {
                    extracted.proxy_version = stored_visitor.proxy_version.clone();
                    extracted.proxy_info = stored_visitor.proxy_info.clone();
                }
                if extracted.gateway_version.is_none() && stored_visitor.gateway_version.is_some() {
                    extracted.gateway_version = stored_visitor.gateway_version.clone();
                    extracted.gateway_info = stored_visitor.gateway_info.clone();
                }
            }

            current_span = span.parent();
        }
    }

    extracted
}

/// Build a version suffix string based on extracted version info
/// 
/// # Arguments
/// * `extracted` - The extracted version information
/// * `own_version` - The application's own version
/// * `own_info` - The application's own system info
/// * `is_error` - Whether this is for an ERROR level log
/// 
/// # Returns
/// A formatted string containing version information suitable for appending to log lines
pub fn build_version_suffix(
    extracted: &ExtractedVersionInfo,
    own_version: &str,
    own_info: &SystemInfo,
    is_error: bool,
) -> String {
    let mut version_suffix = String::new();
    let is_versioned_span = extracted.has_version_info();

    if is_versioned_span || is_error {
        // Own version
        let mut own_version_str = format!(" [{own_version}");
        if is_error {
            own_version_str = format!("{own_version_str} {own_info}");
        }
        own_version_str = format!("{own_version_str}]");
        version_suffix.push_str(&own_version_str);
    }

    // Core version
    if let Some(ref core_version) = extracted.core_version {
        let mut core_version_str = format!("[C:{core_version}");
        if is_error {
            if let Some(ref core_info) = extracted.core_info {
                core_version_str = format!("{core_version_str} {core_info}");
            }
        }
        core_version_str = format!("{core_version_str}]");
        version_suffix.push_str(&core_version_str);
    }

    // Proxy version
    if let Some(ref proxy_version) = extracted.proxy_version {
        let mut proxy_version_str = format!("[PX:{proxy_version}");
        if is_error {
            if let Some(ref proxy_info) = extracted.proxy_info {
                proxy_version_str = format!("{proxy_version_str} {proxy_info}");
            }
        }
        proxy_version_str = format!("{proxy_version_str}]");
        version_suffix.push_str(&proxy_version_str);
    }

    // Gateway version
    if let Some(ref gateway_version) = extracted.gateway_version {
        let mut gateway_version_str = format!("[GW:{gateway_version}");
        if is_error {
            if let Some(ref gateway_info) = extracted.gateway_info {
                gateway_version_str = format!("{gateway_version_str} {gateway_info}");
            }
        }
        gateway_version_str = format!("{gateway_version_str}]");
        version_suffix.push_str(&gateway_version_str);
    }

    version_suffix
}

/// Custom tracing formatter that conditionally includes version information in log messages.
///
/// This formatter wraps the default tracing formatter and adds version suffix to log messages:
/// - For ERROR level logs: includes own_version, own_info and components version and info
/// - For other levels: includes only own_version and proxy_version (if available)
///
/// The version information is extracted from tracing span fields.
pub struct VersionSuffixFormat {
    /// The underlying tracing formatter
    pub inner: tracing_subscriber::fmt::format::Format,
    /// The core application version to display as fallback
    pub own_version: String,
    pub own_info: SystemInfo,
}

/// A layer that captures version fields from spans and stores them for use by the formatter
pub struct VersionFieldLayer;

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

impl<S, N> FormatEvent<S, N> for VersionSuffixFormat
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
        writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        // Extract version information from current span context using utility function
        let extracted = extract_version_info_from_context(ctx);
        
        // Build version suffix using utility function
        let is_error = *event.metadata().level() == Level::ERROR;
        let version_suffix = build_version_suffix(&extracted, &self.own_version, &self.own_info, is_error);

        // Create a wrapper writer that will append version info before newlines
        let mut wrapper = VersionSuffixWriter::new(writer, version_suffix);
        self.inner.format_event(ctx, Writer::new(&mut wrapper), event)
    }
}

/// A wrapper writer that appends version suffix before newlines
pub struct VersionSuffixWriter<'a> {
    inner: Writer<'a>,
    version_suffix: String,
}

impl<'a> VersionSuffixWriter<'a> {
    pub fn new(inner: Writer<'a>, version_suffix: String) -> Self {
        Self { inner, version_suffix }
    }
}

impl<'a> std::fmt::Write for VersionSuffixWriter<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if let Some(content) = s.strip_suffix('\n') {
            // Remove the newline, add version suffix, then add newline back
            writeln!(self.inner, "{}{}", content, self.version_suffix)
        } else {
            // No newline at end, just pass through
            write!(self.inner, "{s}")
        }
    }
}

/// A visitor that extracts version fields from spans
#[derive(Default, Clone)]
pub struct SpanFieldVisitor {
    pub core_version: Option<String>,
    pub core_info: Option<String>,
    pub proxy_version: Option<String>,
    pub proxy_info: Option<String>,
    pub gateway_version: Option<String>,
    pub gateway_info: Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "core_version" => self.core_version = Some(value.to_string()),
            "core_info" => self.core_info = Some(value.to_string()),
            "proxy_version" => self.proxy_version = Some(value.to_string()),
            "proxy_info" => self.proxy_info = Some(value.to_string()),
            "gateway_version" => self.gateway_version = Some(value.to_string()),
            "gateway_info" => self.gateway_info = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "core_version" => self.core_version = Some(format!("{value:?}")),
            "core_info" => self.core_info = Some(format!("{value:?}")),
            "proxy_version" => self.proxy_version = Some(format!("{value:?}")),
            "proxy_info" => self.proxy_info = Some(format!("{value:?}")),
            "gateway_version" => self.gateway_version = Some(format!("{value:?}")),
            "gateway_info" => self.gateway_info = Some(format!("{value:?}")),
            _ => {}
        }
    }
}

/// Custom field formatter that filters out version fields to prevent duplication
pub struct VersionFilteredFields;

impl<'writer> FormatFields<'writer> for VersionFilteredFields {
    fn format_fields<R: RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = FieldFilterVisitor::new(writer);
        fields.record(&mut visitor);
        Ok(())
    }
}

/// Field visitor that skips version-related fields
pub struct FieldFilterVisitor<'writer> {
    writer: Writer<'writer>,
    first: bool,
}

impl<'writer> FieldFilterVisitor<'writer> {
    pub fn new(writer: Writer<'writer>) -> Self {
        Self { writer, first: true }
    }
}

impl<'writer> tracing::field::Visit for FieldFilterVisitor<'writer> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "core_version" | "core_info" | "proxy_version" | "proxy_info" | "gateway_version" | "gateway_info" => {
                // Skip version fields to prevent duplication
            }
            _ => {
                if !self.first {
                    let _ = write!(self.writer, " ");
                }
                let _ = write!(self.writer, "{}={}", field.name(), value);
                self.first = false;
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "core_version" | "core_info" | "proxy_version" | "proxy_info" | "gateway_version" | "gateway_info" => {
                // Skip version fields to prevent duplication
            }
            _ => {
                if !self.first {
                    let _ = write!(self.writer, " ");
                }
                let _ = write!(self.writer, "{}={:?}", field.name(), value);
                self.first = false;
            }
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
                .unwrap_or_else(|_| format!("{log_level},h2=info").into()),
        )
        .with(VersionFieldLayer) // Add custom layer to capture span fields
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .event_format(VersionSuffixFormat {
                    inner: tracing_subscriber::fmt::format::Format::default()
                        .with_ansi(true),
                    own_version: own_version.to_string(),
                    own_info: SystemInfo::get(),
                })
                .fmt_fields(VersionFilteredFields),
        )
        .init();
}
