//! Internal Missing sentinel (spec §2.6) represented inside CEL as a private-use string.
//!
//! **Static paths:** before evaluation, the binding envelope may inject placeholder values under
//! `source` (and related roots) for dotted paths discovered by scanning expressions, so member
//! access on those paths yields a value instead of a CEL engine error.
//!
//! **Engine `NoSuchKey`:** `cel` 0.13 resolves map keys inside the evaluator; there is no hook to
//! turn a missing key on an arbitrary nested expression into Missing without mis-handling cases
//! like `present(source.unknown)` (the argument must error, not become Missing before `present`
//! runs). Treat full engine-level “unknown member → Missing” as out of scope until the CEL
//! integration exposes a safe extension point.

/// Marker never produced by normal JSON ingestion.
pub const MISSING_STR: &str = "\u{E0000}__CEL_MAPPER_MISSING__\u{E0000}";
