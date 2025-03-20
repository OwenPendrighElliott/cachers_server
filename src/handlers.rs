use crate::errors::CacheError;
use crate::request_types::{CreateCacheRequest, DeleteCacheRequest};
use crate::state::AppState;
use actix_web::{web, HttpResponse, Responder};
use cachers::cache::CacheStats;
use cachers::{Cache, FIFOCache, LRUCache, MRUCache, TTLCache};
use std::sync::Arc;
use std::time::Duration;

// POST /cache/create – Create a new named cache.
pub async fn create_cache(
    state: web::Data<AppState>,
    req: web::Json<CreateCacheRequest>,
) -> Result<impl Responder, CacheError> {
    match state.cache_exists(&req.name) {
        Ok(_) => return Err(CacheError::CacheAlreadyExists),
        Err(_) => (),
    }
    let cache: Arc<dyn Cache<String, Vec<u8>>> = match req.cache_type.as_str() {
        "lru" => Arc::new(LRUCache::new(req.capacity)),
        "fifo" => Arc::new(FIFOCache::new(req.capacity)),
        "mru" => Arc::new(MRUCache::new(req.capacity)),
        "ttl" => {
            let ttl_value = Duration::from_secs(req.ttl.unwrap_or(60));
            let check_interval_value = Duration::from_secs(req.check_interval.unwrap_or(10));
            let jitter_value = Duration::from_secs(req.jitter.unwrap_or(0));

            // Assuming your TtlCache has a constructor that accepts these options.
            Arc::new(TTLCache::new(
                ttl_value,
                check_interval_value,
                jitter_value,
                req.capacity,
            ))
        }
        _ => return Err(CacheError::UnknownCacheType),
    };

    match state.insert_cache(req.name.clone(), cache) {
        Ok(_) => Ok(HttpResponse::Ok().body("Cache created")),
        Err(e) => Err(e),
    }
}

// POST /cache/delete – Delete a named cache.
pub async fn delete_cache(
    state: web::Data<AppState>,
    req: web::Json<DeleteCacheRequest>,
) -> Result<impl Responder, CacheError> {
    match state.remove_cache(&req.name) {
        Ok(_) => Ok(HttpResponse::Ok().body("Cache deleted")),
        Err(e) => Err(e),
    }
}

// GET /cache/{cache_name}/{key} – Retrieve a value.
pub async fn get_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
) -> Result<impl Responder, CacheError> {
    let (cache_name, key) = path.into_inner();
    let cache = state.get_cache(&cache_name)?;
    match cache.get(&key) {
        Some(val) => Ok(HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(val.as_ref().clone())),
        None => Err(CacheError::KeyNotFound),
    }
}

// PUT /cache/{cache_name}/{key} – Set a value with raw binary body.
pub async fn set_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
    body: web::Bytes,
) -> Result<impl Responder, CacheError> {
    let (cache_name, key) = path.into_inner();
    let cache = state.get_cache(&cache_name)?;
    cache.set(key, body.to_vec());
    Ok(HttpResponse::Ok().body("Value set"))
}

// DELETE /cache/{cache_name}/{key} – Remove a key.
pub async fn delete_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
) -> Result<impl Responder, CacheError> {
    let (cache_name, key) = path.into_inner();
    let cache = state.get_cache(&cache_name)?;
    cache.remove(&key);
    Ok(HttpResponse::Ok().body("Key removed"))
}

// GET /cache/{cache_name}/stats – Retrieve cache statistics.
pub async fn stats(
    state: web::Data<AppState>,
    cache_name: web::Path<String>,
) -> Result<impl Responder, CacheError> {
    let cache = state.get_cache(&cache_name)?;
    let s: CacheStats = cache.stats();
    let json = format!(
        r#"{{"hits":{},"misses":{},"size":{},"capacity":{}}}"#,
        s.hits, s.misses, s.size, s.capacity
    );
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(json))
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::HeaderValue, test, web, App};
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::request_types::{CreateCacheRequest, DeleteCacheRequest};
    use crate::state::AppState;

    #[macro_export]
    macro_rules! create_app {
        () => {
            test::init_service(
                App::new()
                    .app_data(web::Data::new(AppState {
                        caches: Mutex::new(HashMap::new()),
                    }))
                    .route("/cache/create", web::post().to(create_cache))
                    .route("/cache/delete", web::post().to(delete_cache))
                    .route("/cache/{cache_name}/stats", web::get().to(stats))
                    .route("/cache/{cache_name}/{key}", web::get().to(get_value))
                    .route("/cache/{cache_name}/{key}", web::put().to(set_value))
                    .route("/cache/{cache_name}/{key}", web::delete().to(delete_value)),
            )
            .await
        };
    }

    #[actix_web::test]
    async fn test_create_cache() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_create_cache_already_exists() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_delete_cache() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::post()
            .uri("/cache/delete")
            .set_json(&DeleteCacheRequest {
                name: "test".to_string(),
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_delete_cache_not_found() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/delete")
            .set_json(&DeleteCacheRequest {
                name: "test".to_string(),
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_get_value() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::put()
            .uri("/cache/test/key")
            .set_payload("value")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::get().uri("/cache/test/key").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type"),
            Some(&HeaderValue::from_static("application/octet-stream"))
        );
        let body = test::read_body(resp).await;
        assert_eq!(body.as_ref(), b"value");
    }

    #[actix_web::test]
    async fn test_get_value_not_found() {
        let mut app = create_app!();

        let req = test::TestRequest::get().uri("/cache/test/key").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_set_value() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::put()
            .uri("/cache/test/key")
            .set_payload("value")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_delete_value() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::put()
            .uri("/cache/test/key")
            .set_payload("value")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::delete()
            .uri("/cache/test/key")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_stats() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "lru".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::get()
            .uri("/cache/test/stats")
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type"),
            Some(&HeaderValue::from_static("application/json"))
        );
        let body = test::read_body(resp).await;
        assert_eq!(
            body.as_ref(),
            br#"{"hits":0,"misses":0,"size":0,"capacity":10}"#
        );
    }

    #[actix_web::test]
    async fn test_stats_not_found() {
        let mut app = create_app!();

        let req = test::TestRequest::get()
            .uri("/cache/test/stats")
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_unknown_cache_type() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "unknown".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_create_ttl_cache() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "ttl".to_string(),
                capacity: 10,
                ttl: Some(60),
                check_interval: Some(10),
                jitter: Some(0),
            })
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_create_ttl_cache_defaults() {
        let mut app = create_app!();

        let req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&CreateCacheRequest {
                name: "test".to_string(),
                cache_type: "ttl".to_string(),
                capacity: 10,
                ttl: None,
                check_interval: None,
                jitter: None,
            })
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
