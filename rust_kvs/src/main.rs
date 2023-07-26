use async_trait::async_trait;
use generated::RustKvsService;
use std::collections::HashMap;

mod generated;

struct Kvs {
    values: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl RustKvsService for Kvs {
    async fn store(&mut self, key: String, value: serde_json::Value) -> everest::Result<()> {
        self.values.insert(key, value);
        Ok(())
    }

    async fn load(&mut self, key: String) -> everest::Result<serde_json::Value> {
        Ok(self
            .values
            .get(&key)
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    async fn remove(&mut self, key: String) -> everest::Result<()> {
        self.values.remove(&key);
        Ok(())
    }

    async fn exists(&mut self, key: String) -> everest::Result<bool> {
        Ok(self.values.contains_key(&key))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kvs = Kvs {
        values: HashMap::new(),
    };

    generated::Module::init(kvs).await?.loop_forever().await?;
    Ok(())
}
