//! The performance budgets the Studio's harness asserts on the reference
//! machine (ROADMAP §3.3, R-27; interface 6 of §6.2). The numbers follow
//! §3.3: change both together, never loosening a budget silently (§8.6).
//! CPU-side budgets are checked in CI by the scene and System State tests.

use serde_json::{Value, json};

/// Models this large use the 10k targets.
const LARGE: usize = 5_000;

/// Frame interval p95 during pan and zoom, in milliseconds (C-33).
pub fn frame_p95_ms(elements: usize) -> f64 {
    if elements >= LARGE { 16.7 } else { 8.3 }
}

/// Input to next update p95, in milliseconds.
pub fn input_p95_ms(elements: usize) -> f64 {
    if elements >= LARGE { 16.7 } else { 8.3 }
}

/// Process start to the end of the first update, warm start, in
/// milliseconds (the first interactive frame follows it).
pub const START_TO_FIRST_UPDATE_MS: f64 = 400.0;

/// One budget's line in a report: `met` is `null` when it was not measured.
pub fn result(budget: &str, target_ms: f64, measured_ms: Option<f64>) -> Value {
    json!({
        "budget": budget,
        "target_ms": target_ms,
        "measured_ms": measured_ms,
        "met": measured_ms.map(|measured| measured <= target_ms),
    })
}

/// The budgets a report missed, as `name (measured > target)`.
pub fn missed(results: &[Value]) -> Vec<String> {
    results
        .iter()
        .filter(|result| result["met"] == json!(false))
        .map(|result| {
            format!(
                "{} ({:.2} ms > {} ms)",
                result["budget"].as_str().unwrap_or_default(),
                result["measured_ms"].as_f64().unwrap_or_default(),
                result["target_ms"]
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_follow_the_model_size_and_misses_are_named() {
        assert_eq!(frame_p95_ms(1_000), 8.3);
        assert_eq!(frame_p95_ms(10_000), 16.7);
        let results = [
            result("pan frame p95", 8.3, Some(6.4)),
            result("zoom frame p95", 8.3, Some(9.0)),
            result("start", START_TO_FIRST_UPDATE_MS, None),
        ];
        assert_eq!(missed(&results), ["zoom frame p95 (9.00 ms > 8.3 ms)"]);
        assert_eq!(results[2]["met"], Value::Null);
    }
}
