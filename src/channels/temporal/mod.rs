//! Pure helpers backing the temporal-variety actor.
//!
//! These modules operate on in-memory `Timescales` and JSON buffers to provide
//! lightweight aggregation, pattern, forecast, causality, and visualization
//! calculations. They can be unit-tested without starting the actor tree.

pub mod aggregation;
pub mod causality;
pub mod forecasting;
pub mod patterns;
pub mod timescales;
pub mod visualization;
