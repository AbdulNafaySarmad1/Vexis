//! Stats layer — a first-class module, not an afterthought.
//!
//! Everything here is pure: it takes a finished analysis and produces the
//! `Stats` struct that both the JSON and Markdown reporters consume. Stats are
//! computed exactly once.

pub mod accuracy;
pub mod metrics;

pub use accuracy::{AccuracyReport, Mismatch};
pub use metrics::{CategoryBreakdown, ComplexityEntry, FunctionStats, InstructionStats, Stats};
