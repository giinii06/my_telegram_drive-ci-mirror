//! Offline, read-only compatibility tools for the legacy TDX1 wire contract.
//! Production projection writes remain on the Go implementation until all
//! replay, schema, idempotency, and encryption contracts pass.

pub const WIRE_VERSION: &str = "TDX1";
