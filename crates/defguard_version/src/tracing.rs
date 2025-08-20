//! Tracing integration with version-aware log formatting.
//!
//! This module provides a custom tracing formatter and layer system that automatically
//! includes version and system information in log messages. It's designed to make
//! debugging and monitoring easier in distributed Defguard deployments by providing
//! component version context in logs.
//!
//! # Features
//!
//! - **Version-aware formatting**: Automatically extracts and displays version information
//! - **Component differentiation**: Distinguishes between Core (C:), Proxy (PX:), and Gateway (GW:) components
//! - **Error-level enhancement**: Includes detailed system information for ERROR-level logs
//!
//! # Log Format
//!
//! The formatter adds version suffixes to log messages:
//!
//! - **Regular logs**: `[own_version][C:core_version][PX:proxy_version][GW:gateway_version]`
//! - **Error logs**: `[own_version own_system_info][C:core_version core_info][PX:proxy_version proxy_info][GW:gateway_version gateway_info]`
//!
//! # Span Fields
//!
//! The following span fields are automatically captured and used for version display:
//!
//! - `core_version`, `core_info` - Core component version and system information
//! - `proxy_version`, `proxy_info` - Proxy component version and system information
//! - `gateway_version`, `gateway_info` - Gateway component version and system information
//!
//! # Usage
//!
//! ## Basic Setup
//!
//! ```rust
//! // Initialize tracing with version-aware formatting
//! defguard_version::tracing::init("1.5.0", "info");
//! ```
//!
//! ## Creating Version-Aware Spans
//!
//! ```rust
//! use tracing::info_span;
//!
//! // Create a span with proxy version information
//! let _span = info_span!(
//!     "proxy_communication",
//!     proxy_version = "1.4.2",
//!     proxy_info = "Linux 22.04 64-bit x86_64"
//! ).entered();
//!
//! // This log will include the proxy version information
//! tracing::info!("Processing proxy request");
//! // Output: 2024-01-01T12:00:00Z INFO proxy_communication: Processing proxy request [1.5.0][PX:1.4.2]
//! ```
//!
//! ## Error Logs with Full Context
//!
//! ```rust
//! use tracing::error;
//!
//! // Error logs automatically include system information
//! tracing::error!("Failed to connect to gateway");
//! // Output: 2024-01-01T12:00:00Z ERROR: Failed to connect to gateway [1.5.0 Linux 22.04 64-bit x86_64][GW:1.3.1 Windows 11 64-bit x86_64]
//! ```
//!
//! # Architecture
//!
//! The module implements a layered architecture:
//!
//! 1. **`VersionFieldLayer`** - Captures version fields from spans and stores them in extensions
//! 2. **`VersionSuffixFormat`** - Custom formatter that adds version suffixes to log messages
//! 3. **`VersionFilteredFields`** - Field formatter that excludes version fields from normal output
//! 4. **Utility functions** - Extract and format version information from span hierarchy

use semver::Version;
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    Layer,
    field::RecordFields,
    fmt::{
        FmtContext, FormatEvent, FormatFields,
        format::{Format, Full, Writer},
        time::SystemTime,
    },
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
};

use crate::{ComponentInfo, DefguardVersionError, SystemInfo};

/// Container for version information extracted from tracing span hierarchy.
///
/// Aggregates version and system information for different Defguard components
/// (core, proxy, gateway) found while traversing up the span tree.
#[derive(Debug, Default, Clone)]
pub struct ExtractedVersionInfo {
    pub component: Option<String>,
    pub info: Option<String>,
    pub version: Option<String>,
}

impl ExtractedVersionInfo {
    #[must_use]
    pub fn has_version_info(&self) -> bool {
        self.component.is_some() || self.info.is_some() || self.version.is_some()
    }
}

/// Extract version information from current span context
///
/// This function extracts version information from the current span's extensions
/// that were stored by VersionFieldLayer.
///
/// # Arguments
/// * `ctx` - The format context from the tracing formatter
///
/// # Returns
/// An `ExtractedVersionInfo` struct containing all version information found in the current span
#[must_use]
pub fn extract_version_info_from_context<S, N>(ctx: &FmtContext<'_, S, N>) -> ExtractedVersionInfo
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    let mut extracted = ExtractedVersionInfo::default();

    if let Some(span_ref) = ctx.lookup_current() {
        let extensions = span_ref.extensions();
        if let Some(stored_visitor) = extensions.get::<SpanFieldVisitor>() {
            extracted
                .component
                .clone_from(&stored_visitor.component);
            extracted.version.clone_from(&stored_visitor.version);
            extracted
                .info
                .clone_from(&stored_visitor.info);
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
#[must_use]
pub fn build_version_suffix(
    extracted: &ExtractedVersionInfo,
    own_version: &Version,
    own_info: &SystemInfo,
    is_error: bool,
) -> String {
    let mut version_suffix = String::new();
    let is_versioned_span = extracted.has_version_info();

    if is_versioned_span || is_error {
        // Own version
        version_suffix.push_str(" [");
        version_suffix.push_str(&own_version.to_string());
        if is_error {
            version_suffix.push(' ');
            version_suffix.push_str(&own_info.to_string());
        }
        version_suffix.push(']');
    }

    if let Some(ref component) = extracted.component {
        // TODO enum & match
        let component = component.to_lowercase();
        if component == "core" {
            version_suffix.push_str("[C:");
        } else if component == "proxy" {
            version_suffix.push_str("[PX:");
        } else if component == "gateway" {
            version_suffix.push_str("[GW:");
        }
        if is_error {
            if let Some(ref info) = extracted.info {
                version_suffix.push(' ');
                version_suffix.push_str(&info);
            }
        }
        version_suffix.push(']');
    }

    version_suffix
}

/// Custom tracing formatter that conditionally includes version information in log messages.
///
/// This formatter wraps the default tracing formatter and adds version suffix to log messages:
/// - For ERROR level logs: includes own_version, own_info and components version and info
/// - For other levels: includes only own_version and component version if available
///
/// The version information is extracted from tracing span fields.
pub struct VersionSuffixFormat {
    /// The underlying tracing formatter
    pub inner: tracing_subscriber::fmt::format::Format,
    pub component_info: ComponentInfo,
}

impl VersionSuffixFormat {
    pub fn new(
        own_version: &str,
        inner: Format<Full, SystemTime>,
    ) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            inner,
            component_info: ComponentInfo::new(own_version)?,
        })
    }
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
        let version_suffix = build_version_suffix(
            &extracted,
            &self.component_info.version,
            &self.component_info.system,
            is_error,
        );

        // Create a wrapper writer that will append version info before newlines
        let mut wrapper = VersionSuffixWriter::new(writer, version_suffix);
        self.inner
            .format_event(ctx, Writer::new(&mut wrapper), event)
    }
}

/// A wrapper writer that appends version suffix before newlines
pub struct VersionSuffixWriter<'a> {
    inner: Writer<'a>,
    version_suffix: String,
}

impl<'a> VersionSuffixWriter<'a> {
    #[must_use]
    pub fn new(inner: Writer<'a>, version_suffix: String) -> Self {
        Self {
            inner,
            version_suffix,
        }
    }
}

impl std::fmt::Write for VersionSuffixWriter<'_> {
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
    pub component: Option<String>,
    pub info: Option<String>,
    pub version: Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "component" => self.component = Some(value.to_string()),
            "version" => self.version = Some(value.to_string()),
            "info" => self.info = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "component" => self.component = Some(format!("{value:?}")),
            "version" => self.version = Some(format!("{value:?}")),
            "info" => self.info = Some(format!("{value:?}")),
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

/// Field visitor that skips version-related fields to avoid duplication
pub struct FieldFilterVisitor<'writer> {
    writer: Writer<'writer>,
    first: bool,
}

impl<'writer> FieldFilterVisitor<'writer> {
    #[must_use]
    pub fn new(writer: Writer<'writer>) -> Self {
        Self {
            writer,
            first: true,
        }
    }
}

impl tracing::field::Visit for FieldFilterVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "core_version" | "core_info" | "proxy_version" | "proxy_info" | "gateway_version"
            | "gateway_info" => {
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
            "core_version" | "core_info" | "proxy_version" | "proxy_info" | "gateway_version"
            | "gateway_info" => {
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
pub fn init(own_version: &str, log_level: &str) -> Result<(), DefguardVersionError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{log_level},h2=info").into()),
        )
        .with(VersionFieldLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .event_format(VersionSuffixFormat::new(
                    own_version,
                    tracing_subscriber::fmt::format::Format::default().with_ansi(true),
                )?)
                .fmt_fields(VersionFilteredFields),
        )
        .init();

    Ok(())
}
