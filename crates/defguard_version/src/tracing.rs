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
//! - `component` - component name to use, one of `DefguardComponent` variants
//! - `version` - component version, usually retrieved from the headers
//! - `info` - system information, usually retrieved from the headers
//!
//! # Usage
//!
//! ## Creating Version-Aware Spans
//!
//! ```rust
//! use defguard_version::DefguardComponent;
//! use tracing::info_span;
//!
//! // Create a span with proxy version information
//! let _span = info_span!(
//!     "proxy_communication",
//!     component = %DefguardComponent::Proxy,
//!     version = "1.4.2",
//!     info = "Linux 22.04 64-bit x86_64"
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

use std::{fmt, str::FromStr};

use semver::Version;
use serde::Serialize;
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    EnvFilter, Layer,
    field::RecordFields,
    fmt::{
        FmtContext, FormatEvent, FormatFields,
        format::{Format, Full, Writer},
        time::SystemTime,
    },
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
};

use crate::{ComponentInfo, DefguardComponent, SystemInfo};

/// Container for version information extracted from tracing span hierarchy.
///
/// Aggregates version and system information found while traversing up the span tree.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VersionInfo {
    pub component: Option<DefguardComponent>,
    pub info: Option<String>,
    pub version: Option<String>,
    // FIXME: currently used only in `outdated_components()`.
    pub is_supported: bool,
}

impl VersionInfo {
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
pub fn extract_version_info_from_context<S, N>(ctx: &FmtContext<'_, S, N>) -> VersionInfo
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    let mut extracted = VersionInfo::default();

    if let Some(span_ref) = ctx.lookup_current() {
        let mut current_span = Some(span_ref);
        while let Some(span) = current_span {
            let extensions = span.extensions();

            if let Some(stored_visitor) = extensions.get::<SpanFieldVisitor>() {
                if extracted.component.is_none() && stored_visitor.component.is_some() {
                    extracted.component.clone_from(&stored_visitor.component);
                }
                if extracted.version.is_none() && stored_visitor.version.is_some() {
                    extracted.version.clone_from(&stored_visitor.version);
                }
                if extracted.info.is_none() && stored_visitor.info.is_some() {
                    extracted.info.clone_from(&stored_visitor.info);
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
#[must_use]
pub fn build_version_suffix(
    extracted: &VersionInfo,
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

    if let (Some(component), Some(version)) = (&extracted.component, &extracted.version) {
        match component {
            DefguardComponent::Core => version_suffix.push_str("[C:"),
            DefguardComponent::Proxy => version_suffix.push_str("[PX:"),
            DefguardComponent::Gateway => version_suffix.push_str("[GW:"),
        }
        version_suffix.push_str(version);
        if is_error {
            if let Some(ref info) = extracted.info {
                version_suffix.push(' ');
                version_suffix.push_str(info);
            }
        }
        version_suffix.push(']');
    }

    version_suffix
}

/// Custom tracing formatter that conditionally includes version information in log messages.
///
/// This formatter wraps the default tracing formatter and adds version suffix to log messages:
/// - For ERROR level logs: includes `own_version`, `own_info` and components version and info
/// - For other levels: includes only `own_version` and component version if available
///
/// The version information is extracted from tracing span fields.
pub struct VersionSuffixFormat {
    /// The underlying tracing formatter
    pub inner: Format,
    pub component_info: ComponentInfo,
}

impl VersionSuffixFormat {
    #[must_use]
    pub fn new(own_version: crate::Version, inner: Format<Full, SystemTime>) -> Self {
        Self {
            inner,
            component_info: ComponentInfo::new(own_version),
        }
    }
}

/// A layer that captures version fields from spans and stores them for use by the formatter
pub struct VersionFieldLayer;

impl<S> Layer<S> for VersionFieldLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
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
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    /// Formats a tracing event, conditionally adding version information as a prefix.
    ///
    /// This method includes version information based on:
    /// - For ERROR level logs: includes own and remote component version and system-info
    /// - For other levels: includes only own and remote component version
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        // Extract version information from current span context
        let extracted = extract_version_info_from_context(ctx);

        // Build version suffix
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

impl fmt::Write for VersionSuffixWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Replace newline characters with escaped version to prevent log line splitting
        let escaped = s.replace('\n', "\\n");

        if let Some(content) = escaped.strip_suffix("\\n") {
            // If the original string ended with a newline, add version suffix and restore newline
            writeln!(self.inner, "{content}{}", self.version_suffix)
        } else {
            // No trailing newline, just write the escaped content
            write!(self.inner, "{escaped}")
        }
    }
}

/// A visitor that extracts version fields from spans
#[derive(Default, Clone)]
pub struct SpanFieldVisitor {
    pub component: Option<DefguardComponent>,
    pub info: Option<String>,
    pub version: Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            // "component" => self.component = DefguardComponent::from_str(value).ok(),
            "version" => self.version = Some(value.to_string()),
            "info" => self.info = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");
        match field.name() {
            "component" => self.component = DefguardComponent::from_str(&value).ok(),
            "version" => self.version = Some(value),
            "info" => self.info = Some(value),
            _ => {}
        }
    }
}

/// Custom field formatter that filters out version fields to prevent duplication
pub struct VersionFilteredFields;

impl<'writer> FormatFields<'writer> for VersionFilteredFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
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
            "component" | "info" | "version" => {
                // Skip version fields to prevent duplication
            }
            _ => {
                if !self.first {
                    let _ = write!(self.writer, " ");
                }
                let _ = write!(self.writer, "{}={value}", field.name());
                self.first = false;
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        match field.name() {
            "component" | "info" | "version" => {
                // Skip version fields to prevent duplication
            }
            _ => {
                if !self.first {
                    let _ = write!(self.writer, " ");
                }
                let _ = write!(self.writer, "{}={value:?}", field.name());
                self.first = false;
            }
        }
    }
}

/// Adds a custom formatter that conditionally displays version information to a given subscriber.
///
/// The formatter will:
/// - For ERROR level logs: display own and remote component version and system-info
/// - For other log levels: display only own and remote component version
///
/// Version information is extracted from tracing span fields with names:
/// - `component` - component name to use, one of `DefguardComponent` variants
/// - `version` - component version, usually retrieved from the headers
/// - `info` - system information, usually retrieved from the headers
///
/// # Arguments
/// * `own_version` - The application semantic version
/// * `log_level` - The log level filter to use
///
/// # Examples
/// ```
/// let subscriber = tracing_subscriber::registry();
/// defguard_version::tracing::with_version_formatters(
///     &defguard_version::Version::new(1, 5, 0),
///     "info",
///     subscriber,
/// );
/// ```
pub fn with_version_formatters<S>(
    own_version: &crate::Version,
    log_level: &str,
    subscriber: S,
) -> impl tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync,
{
    subscriber
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{log_level},h2=info").into()),
        )
        .with(VersionFieldLayer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .event_format(VersionSuffixFormat::new(
                    crate::Version::new(own_version.major, own_version.minor, own_version.patch),
                    Format::default().with_ansi(true),
                ))
                .fmt_fields(VersionFilteredFields),
        )
}
