//! Shared facades and pure helper modules.
//!
//! `shared` contains compatibility-style wrappers around the channel and
//! message APIs plus pure modules for recursive viable-system structures and
//! variety engineering. Pure helpers can be used without starting the actor
//! application.

pub mod channel;
pub mod message;
pub mod recursion;
pub mod variety;
pub mod variety_engineering;
