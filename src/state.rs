use crate::errors::CacheError;
use cachers::Cache;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub caches: Mutex<HashMap<String, Arc<dyn Cache<String, Vec<u8>> + Send + Sync>>>,
}

impl AppState {
    pub fn get_cache(
        &self,
        name: &str,
    ) -> Result<Arc<dyn Cache<String, Vec<u8>> + Send + Sync>, CacheError> {
        let caches = self.caches.lock().map_err(|_| CacheError::Internal)?;
        caches.get(name).cloned().ok_or(CacheError::CacheNotFound)
    }

    pub fn remove_cache(
        &self,
        name: &str,
    ) -> Result<Arc<dyn Cache<String, Vec<u8>> + Send + Sync>, CacheError> {
        let mut caches = self.caches.lock().map_err(|_| CacheError::Internal)?;
        caches.remove(name).ok_or(CacheError::CacheNotFound)
    }

    pub fn insert_cache(
        &self,
        name: String,
        cache: Arc<dyn Cache<String, Vec<u8>> + Send + Sync>,
    ) -> Result<(), CacheError> {
        let mut caches = self.caches.lock().map_err(|_| CacheError::Internal)?;
        if caches.contains_key(&name) {
            return Err(CacheError::CacheAlreadyExists);
        }
        caches.insert(name, cache);
        Ok(())
    }

    pub fn cache_exists(&self, name: &str) -> Result<(), CacheError> {
        let caches = self.caches.lock().unwrap();
        match caches.contains_key(name) {
            true => Ok(()),
            false => Err(CacheError::CacheNotFound),
        }
    }
}
