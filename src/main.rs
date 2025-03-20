use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use cachers::cache::CacheStats;
use cachers::{Cache, FIFOCache, LRUCache, MRUCache, TTLCache};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
// Global state: a map of named caches.
struct AppState {
    caches: Mutex<HashMap<String, Arc<dyn Cache<String, Vec<u8>>>>>,
}

// Request for creating a cache.
#[derive(Debug, Deserialize)]
struct CreateCacheRequest {
    name: String,
    cache_type: String,
    capacity: u64,
    #[serde(default)]
    ttl: Option<u64>,
    #[serde(default)]
    check_interval: Option<u64>,
    #[serde(default)]
    jitter: Option<u64>,
}

// Request for deleting a cache.
#[derive(Debug, Deserialize)]
struct DeleteCacheRequest {
    name: String,
}

// POST /cache/create – Create a new named cache.
async fn create_cache(
    state: web::Data<AppState>,
    req: web::Json<CreateCacheRequest>,
) -> impl Responder {
    let mut caches = state.caches.lock().unwrap();
    if caches.contains_key(&req.name) {
        return HttpResponse::BadRequest().body("Cache with that name already exists");
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
        _ => return HttpResponse::BadRequest().body("Unknown cache type"),
    };
    caches.insert(req.name.clone(), cache);
    HttpResponse::Ok().body("Cache created")
}

// POST /cache/delete – Delete a named cache.
async fn delete_cache(
    state: web::Data<AppState>,
    req: web::Json<DeleteCacheRequest>,
) -> impl Responder {
    let mut caches = state.caches.lock().unwrap();
    if caches.remove(&req.name).is_none() {
        return HttpResponse::NotFound().body("Cache not found");
    }
    HttpResponse::Ok().body("Cache deleted")
}

// GET /cache/{cache_name}/{key} – Retrieve a value.
async fn get_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
) -> impl Responder {
    let (cache_name, key) = path.into_inner();
    let caches = state.caches.lock().unwrap();
    let cache = match caches.get(&cache_name) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Cache not found"),
    };
    match cache.get(&key) {
        Some(val) => HttpResponse::Ok().body(val.as_ref().clone()),
        None => HttpResponse::NotFound().body("Key not found"),
    }
}

// PUT /cache/{cache_name}/{key} – Set a value with raw binary body.
async fn set_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
    body: web::Bytes,
) -> impl Responder {
    let (cache_name, key) = path.into_inner();
    let caches = state.caches.lock().unwrap();
    let cache = match caches.get(&cache_name) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Cache not found"),
    };
    cache.set(key, body.to_vec());
    HttpResponse::Ok().body("Value set")
}

// DELETE /cache/{cache_name}/{key} – Remove a key.
async fn delete_value(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (cache_name, key)
) -> impl Responder {
    let (cache_name, key) = path.into_inner();
    let caches = state.caches.lock().unwrap();
    let cache = match caches.get(&cache_name) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Cache not found"),
    };
    cache.remove(&key);
    HttpResponse::Ok().body("Key removed")
}

// GET /cache/{cache_name}/stats – Retrieve cache statistics.
async fn stats(state: web::Data<AppState>, cache_name: web::Path<String>) -> impl Responder {
    let caches = state.caches.lock().unwrap();
    let cache = match caches.get(&cache_name.into_inner()) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Cache not found"),
    };
    let s: CacheStats = cache.stats();
    let json = format!(
        r#"{{"hits":{},"misses":{},"size":{},"capacity":{}}}"#,
        s.hits, s.misses, s.size, s.capacity
    );
    HttpResponse::Ok()
        .content_type("application/json")
        .body(json)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        caches: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/cache/create", web::post().to(create_cache))
            .route("/cache/delete", web::post().to(delete_cache))
            .route("/cache/{cache_name}/stats", web::get().to(stats))
            .route("/cache/{cache_name}/{key}", web::get().to(get_value))
            .route("/cache/{cache_name}/{key}", web::put().to(set_value))
            .route("/cache/{cache_name}/{key}", web::delete().to(delete_value))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http, test, App};
    use serde_json::json;

    #[actix_web::test]
    async fn integration_test() {
        // Create shared app state.
        let state = actix_web::web::Data::new(AppState {
            caches: std::sync::Mutex::new(std::collections::HashMap::new()),
        });

        // Initialize the app with all routes.
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .route("/cache/create", web::post().to(create_cache))
                .route("/cache/{cache_name}/stats", web::get().to(stats))
                .route("/cache/{cache_name}/{key}", web::get().to(get_value))
                .route("/cache/{cache_name}/{key}", web::put().to(set_value))
                .route("/cache/{cache_name}/{key}", web::delete().to(delete_value)),
        )
        .await;

        // Create a cache named "test_cache" of type "lru".
        let create_req = test::TestRequest::post()
            .uri("/cache/create")
            .set_json(&json!({
                "name": "test_cache",
                "cache_type": "lru",
                "capacity": 10
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), http::StatusCode::OK);

        // Set key "foo" to value "bar".
        let put_req = test::TestRequest::put()
            .uri("/cache/test_cache/foo")
            .set_payload("bar")
            .to_request();
        let put_resp = test::call_service(&app, put_req).await;
        assert_eq!(put_resp.status(), http::StatusCode::OK);

        // Retrieve the value for key "foo".
        let get_req = test::TestRequest::get()
            .uri("/cache/test_cache/foo")
            .to_request();
        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), http::StatusCode::OK);
        let body = test::read_body(get_resp).await;
        assert_eq!(body, actix_web::web::Bytes::from("bar"));

        // Remove key "foo".
        let delete_req = test::TestRequest::delete()
            .uri("/cache/test_cache/foo")
            .to_request();
        let delete_resp = test::call_service(&app, delete_req).await;
        assert_eq!(delete_resp.status(), http::StatusCode::OK);

        // Confirm key "foo" no longer exists.
        let get_req2 = test::TestRequest::get()
            .uri("/cache/test_cache/foo")
            .to_request();
        let get_resp2 = test::call_service(&app, get_req2).await;
        assert_eq!(get_resp2.status(), http::StatusCode::NOT_FOUND);

        // Check cache statistics.
        let stats_req = test::TestRequest::get()
            .uri("/cache/test_cache/stats")
            .to_request();
        let stats_resp = test::call_service(&app, stats_req).await;
        assert_eq!(stats_resp.status(), http::StatusCode::OK);
        let stats_body = test::read_body(stats_resp).await;
        let stats_json: serde_json::Value = serde_json::from_slice(&stats_body).unwrap();
        assert!(stats_json.get("hits").is_some());
        assert!(stats_json.get("misses").is_some());
        assert!(stats_json.get("size").is_some());
        assert!(stats_json.get("capacity").is_some());
    }
}
