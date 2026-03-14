//! OTLP trace export setup (behind the `otlp-export` Cargo feature flag).
//!
//! When the `ENGRAM_OTLP_ENDPOINT` environment variable is set and the
//! `otlp-export` feature is compiled in, this module builds a
//! `tracing-opentelemetry` layer that the caller can attach to an
//! existing subscriber via `tracing_subscriber::layer::SubscriberExt`.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 2 for requirements.

/// Build an OTLP gRPC tracing layer pointing at `endpoint`.
///
/// Returns a `Box<dyn Layer>` that the caller should attach to the global
/// subscriber via `.with(layer)` — this function does **not** install its
/// own global subscriber, preserving composability with existing fmt/filter
/// layers set up by `init_tracing`.
///
/// # Errors
///
/// Returns an error if the exporter cannot be built.
#[cfg(feature = "otlp-export")]
pub fn build_otlp_layer(
    endpoint: &str,
) -> Result<
    Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>,
    Box<dyn std::error::Error + Send + Sync>,
> {
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

    Ok(Box::new(telemetry))
}

/// No-op when the `otlp-export` feature is not compiled in.
///
/// Returns `Ok(())`, signalling the caller that no OTLP layer is available.
#[cfg(not(feature = "otlp-export"))]
pub fn build_otlp_layer(_endpoint: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
