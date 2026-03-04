//! Event debouncer: `notify-debouncer-full` integration.
//!
//! Wraps the file watcher output in a configurable debounce window (default 500 ms).
//! Collapsed events are emitted as `WatcherEvent` values and routed to the
//! code graph and embedding services.

// TODO(T041): implement notify-debouncer-full with configurable duration
// TODO(T042): wire debounced events to existing pipelines (code_graph, embeddings)
// TODO(T092): implement WatcherEvent→service adapter
