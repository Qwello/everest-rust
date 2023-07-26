use crate::everest;
use async_trait::async_trait;

/// This interface defines a simple key-value-store interface
// TODO(sirver): This is made an async trait, because it is possible that further RPCs are needed
// in the general case, which gets awkward quickly.
#[async_trait]
pub trait RustKvsService {
    const METADATA: &'static str =
        "{\"module\":\"RustKvs\",\"provides\":{\"main\":{\"interface\":\"kvs\"}}}";

    // TODO(sirver): Should also generate documentation
    // TODO(sirver): Should generate pattern matching for key before it being called.
    // TODO(sirver): How to generalize value generation?
    // TODO(sirver): I think type can be more general, maybe every type in types?
    async fn store(&mut self, key: String, value: serde_json::Value) -> everest::Result<()>;

    /// This command loads the previously stored value for a given key (it will return null if the key does not exist)
    async fn load(&mut self, key: String) -> everest::Result<serde_json::Value>;

    /// This command removes the value stored under a given key.
    async fn remove(&mut self, key: String) -> everest::Result<()>;

    /// This command checks if something is stored under a given key.
    async fn exists(&mut self, key: String) -> everest::Result<bool>;
}
