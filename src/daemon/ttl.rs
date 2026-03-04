//! Idle TTL timer: activity tracking and automatic daemon shutdown.
//!
//! Tracks the timestamp of the most recent activity (tool call or file event).
//! A background task periodically checks whether the idle duration has exceeded
//! the configured timeout and triggers graceful shutdown when it has.

// TODO(T048): implement idle TTL timer — activity tracking, periodic expiry check
