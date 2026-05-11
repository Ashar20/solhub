//! Yellowstone gRPC client stub (live-net feature only).
//!
//! Full implementation requires the `yellowstone-grpc-client` crate and a
//! live Triton/Yellowstone endpoint. See IDEA.md §5.2.

/// Placeholder struct — will hold the gRPC client once wired.
pub struct YellowstoneClient {
    pub endpoint: String,
    pub x_token: String,
}

impl YellowstoneClient {
    pub fn new(endpoint: impl Into<String>, x_token: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            x_token: x_token.into(),
        }
    }
}
