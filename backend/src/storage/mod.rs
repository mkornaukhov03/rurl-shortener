mod redis;

use crate::config;

mod internal {
    use crate::config;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    #[allow(unused)]
    pub enum StorageInner {
        NonPersistent(RwLock<HashMap<String, String>>),
        Redis(crate::storage::redis::RedisSingleConnection),
    }

    impl StorageInner {
        pub async fn store(&self, short: String, url: String) -> bool {
            match self {
                StorageInner::NonPersistent(rw_lock) => {
                    let mut guard = rw_lock.write().await;
                    if let std::collections::hash_map::Entry::Vacant(e) = guard.entry(short) {
                        e.insert(url);
                        true
                    } else {
                        false
                    }
                }
                StorageInner::Redis(redis_single_connection) => {
                    redis_single_connection.store(short, url).await
                }
            }
        }

        pub async fn fetch(&self, short: &str) -> Option<String> {
            match self {
                StorageInner::NonPersistent(rw_lock) => rw_lock.read().await.get(short).cloned(),
                StorageInner::Redis(redis_single_connection) => {
                    redis_single_connection.fetch(short).await
                }
            }
        }

        pub async fn from_config(config: &config::Config) -> Self {
            match &config.redis_endpoint {
                Some(endpoint) => StorageInner::Redis(
                    crate::storage::redis::RedisSingleConnection::new(endpoint.to_string()).await,
                ),
                None => StorageInner::NonPersistent(RwLock::new(HashMap::default())),
            }
        }
    }
}

pub struct Storage(internal::StorageInner);

impl Storage {
    pub async fn store(&self, short: String, url: String) -> bool {
        self.0.store(short, url).await
    }

    pub async fn fetch(&self, short: &str) -> Option<String> {
        self.0.fetch(short).await
    }

    pub async fn from_config(config: &config::Config) -> Self {
        Storage(internal::StorageInner::from_config(config).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage() {
        let storage = internal::StorageInner::NonPersistent(Default::default());
        assert!(storage.store("key".into(), "val".into()).await);
        assert!(!storage.store("key".into(), "val2".into()).await);
        assert!(storage.fetch("key").await == Some("val".into()));
    }
}
