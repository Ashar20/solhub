//! Jito ShredStream client stub (live-net feature only).
//!
//! Full implementation requires a live Jito ShredStream proxy. See IDEA.md §5.

/// Placeholder struct — will hold the ShredStream connection once wired.
pub struct ShredStreamClient {
    pub endpoint: String,
}

impl ShredStreamClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}
