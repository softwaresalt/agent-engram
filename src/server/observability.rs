//! OTLP trace export setup (behind the `otlp-export` Cargo feature flag).
//!
//! When the `ENGRAM_OTLP_ENDPOINT` environment variable is set and the
//! `otlp-export` feature is compiled in, this module configures a
//! `tracing-opentelemetry` layer on the global tracing subscriber that
//! exports spans to the configured OpenTelemetry collector via gRPC.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 2 for requirements.
