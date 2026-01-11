//! Implements x86-64 specifics definitions and naive instruction wrappers.
//!
//! This module focuses on providing as "raw" definitions and wrappers as
//! possible. Abstractions and convenient logics to deal with them should be
//! implemented outside this module.
//!
//! This module does not intend to provide complete or exhaustive definitions of
//! registers. Some may be defined in nearly completely, some others may be
//! defined partially, and some may not be defined at all.

pub(crate) mod control_registers;
pub(crate) mod misc;
pub(crate) mod msr;
pub(crate) mod segment;
pub(crate) mod vmx;
