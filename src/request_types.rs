use serde::{Deserialize, Serialize};

// Request for creating a cache.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCacheRequest {
    pub name: String,
    pub cache_type: String,
    pub capacity: u64,
    #[serde(default)]
    pub ttl: Option<u64>,
    #[serde(default)]
    pub check_interval: Option<u64>,
    #[serde(default)]
    pub jitter: Option<u64>,
}

// Request for deleting a cache.
#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteCacheRequest {
    pub name: String,
}
