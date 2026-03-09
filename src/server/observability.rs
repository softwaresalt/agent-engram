//! OTLP trace export setup (behind the `otlp-export` Cargo feature flag).
//!
//! When the `ENGRAM_OTLP_ENDPOINT` environment variable is set and the
//! `otlp-export` feature is compiled in, this module configures a
//! `tracing-opentelemetry` layer on the global tracing subscriber that
//! exports spans to the configured OpenTelemetry collector via gRPC.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 2 for requirements.

/// Configure and install an OTLP gRPC span exporter pointing at `endpoint`.
///
/// This is a no-op when the `otlp-export` Cargo feature is not compiled in.
///
/// # Errors
///
/// Returns an error if the exporter cannot be built or if installing the
/// global tracing subscriber fails (e.g., one was already installed).
#[cfg(feature = "otlp-export")]
pub fn init_otlp_layer(endpoint: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "engram");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    use tracing_subscriber::layer::SubscriberExt;
    let subscriber = tracing_subscriber::registry().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// No-op when the `otlp-export` feature is not compiled in.
#[cfg(not(feature = "otlp-export"))]
pub fn init_otlp_layer(_endpoint: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
