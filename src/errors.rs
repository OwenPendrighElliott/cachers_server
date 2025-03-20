use actix_web::{HttpResponse, ResponseError};
use derive_more::Display;

#[derive(Debug, Display)]
pub enum CacheError {
    #[display("Cache not found")]
    CacheNotFound,
    #[display("Cache already exists")]
    CacheAlreadyExists,
    #[display("Unknown cache type")]
    UnknownCacheType,
    #[display("Key not found")]
    KeyNotFound,
    #[display("Internal error")]
    Internal,
}

impl ResponseError for CacheError {
    fn error_response(&self) -> HttpResponse {
        match self {
            CacheError::CacheNotFound => HttpResponse::NotFound().body("Cache not found"),
            CacheError::CacheAlreadyExists => HttpResponse::Conflict().body("Cache already exists"),
            CacheError::UnknownCacheType => HttpResponse::BadRequest().body("Unknown cache type"),
            CacheError::KeyNotFound => HttpResponse::NotFound().body("Key not found"),
            CacheError::Internal => HttpResponse::InternalServerError().body("Internal error"),
        }
    }
}
