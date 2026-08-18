//! Token-bucket rate limiter for AI tool calls.
//!
//! Caps the number of actions the AI agent can take per minute (default 30).
//! This prevents runaway loops from spamming the user's machine if the AI
//! decides to call `shell` 100 times in a second.
//!
//! The limiter is process-wide (a single bucket shared by every agent run).
//! This is intentional: even with multiple concurrent conversations, the
//! aggregate rate stays bounded.

use std::time::Instant;

use parking_lot::Mutex;

const REFILL_PER_SEC: u64 = 1; // refill 1 token per second = 60/min max
const BURST_CAPACITY: u64 = 30; // burst up to 30 in a row before throttling

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

static BUCKET: Mutex<Option<Bucket>> = parking_lot::const_mutex(None);

/// Try to consume one token. Returns true if allowed, false if rate-limited.
pub fn try_consume() -> bool {
    let mut guard = BUCKET.lock();
    let bucket = guard.get_or_insert_with(|| Bucket {
        tokens: BURST_CAPACITY as f64,
        last_refill: Instant::now(),
    });
    let now = Instant::now();
    let elapsed = now.duration_since(bucket.last_refill);
    let refilled = elapsed.as_secs_f64() * REFILL_PER_SEC as f64;
    bucket.tokens = (bucket.tokens + refilled).min(BURST_CAPACITY as f64);
    bucket.last_refill = now;
    if bucket.tokens >= 1.0 {
        bucket.tokens -= 1.0;
        true
    } else {
        false
    }
}

/// Reset the bucket to full. Used in tests and after explicit user override.
pub fn reset() {
    let mut guard = BUCKET.lock();
    *guard = Some(Bucket {
        tokens: BURST_CAPACITY as f64,
        last_refill: Instant::now(),
    });
}

/// Estimated remaining tokens (for diagnostics).
pub fn available_tokens() -> f64 {
    let mut guard = BUCKET.lock();
    let bucket = guard.get_or_insert_with(|| Bucket {
        tokens: BURST_CAPACITY as f64,
        last_refill: Instant::now(),
    });
    let now = Instant::now();
    let elapsed = now.duration_since(bucket.last_refill);
    let refilled = elapsed.as_secs_f64() * REFILL_PER_SEC as f64;
    (bucket.tokens + refilled).min(BURST_CAPACITY as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_throttles() {
        reset();
        // Allow up to BURST_CAPACITY immediate calls.
        for _ in 0..BURST_CAPACITY {
            assert!(try_consume(), "should allow within burst capacity");
        }
        // The next call should be denied (no tokens left, no time elapsed).
        assert!(!try_consume(), "should deny after burst exhausted");
    }
}
