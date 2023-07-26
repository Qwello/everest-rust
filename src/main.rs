use argh::FromArgs;
use async_trait::async_trait;
use generated::RustKvsService;
use rumqttc::{self, AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

mod everest;
mod generated;

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// prefix of installation.
    #[argh(option)]
    #[allow(unused)]
    prefix: PathBuf,

    /// configuration yml that we are running.
    #[argh(option)]
    #[allow(unused)]
    conf: PathBuf,

    /// module name for us.
    #[argh(option)]
    module: String,
}

struct Publisher {
    client: AsyncClient,
    module: String,
}

impl Publisher {
    pub async fn publish(&self, topic: &str, value: impl Into<Vec<u8>>) -> everest::Result<()> {
        self.client
            .publish(
                &format!("everest/{}/{topic}", self.module),
                QoS::ExactlyOnce,
                false,
                value,
            )
            .await?;
        Ok(())
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
enum Command {
    Call { name: String, data: CallData },
    Result { name: String, data: ResultData },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallData {
    id: String,
    origin: String,
    args: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResultData {
    id: String,
    origin: String,
    retval: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    // Parse the socket address.
    let mqtt_socket: SocketAddr = "127.0.0.1:1883".parse().unwrap();

    // Setup the mqtt client.
    let mut mqtt_options = MqttOptions::new(
        format!("RustKvs/{}", args.module),
        mqtt_socket.ip().to_string(),
        mqtt_socket.port(),
    );
    mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));

    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    let topic = format!("everest/{}/main/cmd", args.module);
    println!("#sirver topic: {:#?}", topic);
    client.subscribe(topic.clone(), QoS::ExactlyOnce).await?;

    let publisher = Publisher {
        client: client.clone(),
        module: args.module.clone(),
    };

    publisher.publish("metadata", Kvs::METADATA).await?;

    let mut kvs = Kvs {
        values: HashMap::new(),
    };

    // Subscribe to the `COMMAND_TOPIC`.
    publisher.publish("ready", "true").await?;

    loop {
        // Unwrap the event.
        let msg = event_loop.poll().await?;
        match msg {
            Event::Incoming(Packet::Publish(data)) => {
                // TODO(sirver): This exists the program on invalid data. Is this desired?
                let cmd: Command = serde_json::from_slice(&data.payload)?;
                let (name, mut data) = match cmd {
                    Command::Call { name, data } => (name, data),
                    Command::Result { .. } => continue,
                };
                match &name as &str {
                    "store" => {
                        let key: String = serde_json::from_value(
                            data.args
                                .remove("key")
                                .ok_or(everest::Error::MissingArgument("key"))?,
                        )?;
                        let value: serde_json::Value = serde_json::from_value(
                            data.args
                                .remove("value")
                                .ok_or(everest::Error::MissingArgument("value"))?,
                        )?;
                        #[allow(clippy::let_unit_value)]
                        let retval = kvs.store(key, value).await?;
                        publisher
                            .publish(
                                "main/cmd",
                                serde_json::to_string(&Command::Result {
                                    name,
                                    data: ResultData {
                                        id: data.id,
                                        origin: args.module.clone(),
                                        retval: {
                                            #[allow(clippy::useless_conversion)]
                                            retval.into()
                                        },
                                    },
                                })?,
                            )
                            .await?;
                    }
                    "load" => {
                        let key: String = serde_json::from_value(
                            data.args
                                .remove("key")
                                .ok_or(everest::Error::MissingArgument("key"))?,
                        )?;
                        #[allow(clippy::let_unit_value)]
                        let retval = kvs.load(key).await?;
                        publisher
                            .publish(
                                "main/cmd",
                                serde_json::to_string(&Command::Result {
                                    name,
                                    data: ResultData {
                                        id: data.id,
                                        origin: args.module.clone(),

                                        retval: {
                                            #[allow(clippy::useless_conversion)]
                                            retval.into()
                                        },
                                    },
                                })?,
                            )
                            .await?;
                    }
                    "remove" => {
                        let key: String = serde_json::from_value(
                            data.args
                                .remove("key")
                                .ok_or(everest::Error::MissingArgument("key"))?,
                        )?;
                        #[allow(clippy::let_unit_value)]
                        let retval = kvs.remove(key).await?;
                        publisher
                            .publish(
                                "main/cmd",
                                serde_json::to_string(&Command::Result {
                                    name,
                                    data: ResultData {
                                        id: data.id,
                                        origin: args.module.clone(),
                                        retval: {
                                            #[allow(clippy::useless_conversion)]
                                            retval.into()
                                        },
                                    },
                                })?,
                            )
                            .await?;
                    }
                    "exists" => {
                        let key: String = serde_json::from_value(
                            data.args
                                .remove("key")
                                .ok_or(everest::Error::MissingArgument("key"))?,
                        )?;
                        #[allow(clippy::let_unit_value)]
                        let retval = kvs.exists(key).await?;
                        publisher
                            .publish(
                                "main/cmd",
                                serde_json::to_string(&Command::Result {
                                    name,
                                    data: ResultData {
                                        id: data.id,
                                        origin: args.module.clone(),
                                        retval: {
                                            #[allow(clippy::useless_conversion)]
                                            retval.into()
                                        },
                                    },
                                })?,
                            )
                            .await?;
                    }
                    _ => {
                        // Everest ignores unknown commands without error message.
                    }
                }
            }
            Event::Outgoing(_) | Event::Incoming(_) => (),
        }
    }
}
