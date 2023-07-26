use async_trait::async_trait;
use everest::{Command, Result, ResultData};
use rumqttc::{self, AsyncClient, Event, EventLoop, Packet, QoS};
use std::collections::HashSet;

const METADATA: &str = "{\"module\":\"RustKvs\",\"provides\":{\"main\":{\"interface\":\"kvs\"}}}";

/// This interface defines a simple key-value-store interface
// TODO(sirver): This is made an async trait, because it is possible that further RPCs are needed
// in the general case, which gets awkward quickly.
#[async_trait]
pub trait RustKvsService {
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

mod kvs_service {
    use super::*;

    pub fn generate_topics(module_name: &str) -> HashSet<String> {
        let mut rv = HashSet::new();
        rv.insert(format!("everest/{}/main/cmd", module_name));
        rv
    }

    pub async fn handle_mqtt_message<KvsService: RustKvsService>(
        module: &mut Module<KvsService>,
        payload: &[u8],
    ) -> Result<()> {
        // TODO(sirver): This quietly ignores wrong input.
        let Ok(cmd) = serde_json::from_slice::<Command>(payload) else {
            return Ok(());
        };
        let (name, mut data) = match cmd {
            Command::Call { name, data } => (name, data),
            Command::Result { .. } => return Ok(()),
        };
        match &name as &str {
            "store" => {
                let key: String = serde_json::from_value(
                    data.args
                        .remove("key")
                        .ok_or(everest::Error::MissingArgument("key"))?,
                )
                .map_err(|_| everest::Error::InvalidArgument("key"))?;
                let value: serde_json::Value = serde_json::from_value(
                    data.args
                        .remove("value")
                        .ok_or(everest::Error::MissingArgument("value"))?,
                )
                .map_err(|_| everest::Error::InvalidArgument("value"))?;
                #[allow(clippy::let_unit_value)]
                let retval = module.kvs_service.store(key, value).await?;
                module
                    .publish(
                        "main/cmd",
                        serde_json::to_string(&Command::Result {
                            name,
                            data: ResultData {
                                id: data.id,
                                origin: module.module_name.clone(),
                                retval: {
                                    #[allow(clippy::useless_conversion)]
                                    retval.into()
                                },
                            },
                        })
                        .expect("serialization should be infallible for this data type"),
                    )
                    .await?;
            }
            "load" => {
                let key: String = serde_json::from_value(
                    data.args
                        .remove("key")
                        .ok_or(everest::Error::MissingArgument("key"))?,
                )
                .map_err(|_| everest::Error::InvalidArgument("key"))?;
                #[allow(clippy::let_unit_value)]
                let retval = module.kvs_service.load(key).await?;
                module
                    .publish(
                        "main/cmd",
                        serde_json::to_string(&Command::Result {
                            name,
                            data: ResultData {
                                id: data.id,
                                origin: module.module_name.clone(),

                                retval: {
                                    #[allow(clippy::useless_conversion)]
                                    retval.into()
                                },
                            },
                        })
                        .expect("serialization should be infallible for this data type"),
                    )
                    .await?;
            }
            "remove" => {
                let key: String = serde_json::from_value(
                    data.args
                        .remove("key")
                        .ok_or(everest::Error::MissingArgument("key"))?,
                )
                .map_err(|_| everest::Error::InvalidArgument("key"))?;
                #[allow(clippy::let_unit_value)]
                let retval = module.kvs_service.remove(key).await?;
                module
                    .publish(
                        "main/cmd",
                        serde_json::to_string(&Command::Result {
                            name,
                            data: ResultData {
                                id: data.id,
                                origin: module.module_name.clone(),
                                retval: {
                                    #[allow(clippy::useless_conversion)]
                                    retval.into()
                                },
                            },
                        })
                        .expect("serialization should be infallible for this data type"),
                    )
                    .await?;
            }
            "exists" => {
                let key: String = serde_json::from_value(
                    data.args
                        .remove("key")
                        .ok_or(everest::Error::MissingArgument("key"))?,
                )
                .map_err(|_| everest::Error::InvalidArgument("key"))?;
                #[allow(clippy::let_unit_value)]
                let retval = module.kvs_service.exists(key).await?;
                module
                    .publish(
                        "main/cmd",
                        serde_json::to_string(&Command::Result {
                            name,
                            data: ResultData {
                                id: data.id,
                                origin: module.module_name.clone(),
                                retval: {
                                    #[allow(clippy::useless_conversion)]
                                    retval.into()
                                },
                            },
                        })
                        .expect("serialization should be infallible for this data type"),
                    )
                    .await?;
            }
            _ => {
                // Everest ignores unknown commands without error message.
            }
        }
        Ok(())
    }
}

pub struct Module<KvsService: RustKvsService> {
    client: AsyncClient,
    event_loop: EventLoop,
    module_name: String,
    kvs_service: KvsService,
    kvs_service_topics: HashSet<String>,
}

impl<KvsService: RustKvsService> Module<KvsService> {
    pub async fn init(kvs_service: KvsService) -> Result<Self> {
        let (client, event_loop, module_name) = everest::initialize_mqtt("RustKvs");

        let kvs_service_topics = kvs_service::generate_topics(&module_name);
        for t in &kvs_service_topics {
            client.subscribe(t, QoS::ExactlyOnce).await?;
        }

        let m = Module {
            client,
            event_loop,
            kvs_service,
            kvs_service_topics,
            module_name,
        };
        m.publish("metadata", METADATA).await?;

        m.publish("ready", "true").await?;
        Ok(m)
    }

    async fn publish(&self, topic: &str, value: impl Into<Vec<u8>>) -> Result<()> {
        self.client
            .publish(
                &format!("everest/{}/{topic}", self.module_name),
                QoS::ExactlyOnce,
                false,
                value,
            )
            .await?;
        Ok(())
    }

    pub async fn loop_forever(&mut self) -> Result<()> {
        loop {
            // Unwrap the event.
            let msg = self.event_loop.poll().await?;
            match msg {
                Event::Incoming(Packet::Publish(data)) => {
                    if self.kvs_service_topics.contains(&data.topic) {
                        kvs_service::handle_mqtt_message(self, &data.payload).await?;
                    }
                }
                Event::Outgoing(_) | Event::Incoming(_) => (),
            }
        }
    }
}
