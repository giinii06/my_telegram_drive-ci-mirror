//! Read-only legacy TDX1 compatibility tools. No production writes.
mod wire;

pub use wire::{OpType, Operation, WireError, format_simple_wire, parse_wire};

pub const WIRE_VERSION: &str = "TDX1";
