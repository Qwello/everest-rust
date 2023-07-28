use argh::FromArgs;
use rumqttc::{self, AsyncClient, EventLoop, MqttOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("mqtt error")]
    MqttClient(#[from] rumqttc::ClientError),
    #[error("mqtt error")]
    MqttConnection(#[from] rumqttc::ConnectionError),
    #[error("missing argument to command call: '{0}'")]
    MissingArgument(&'static str),
    #[error("invalid argument to command call: '{0}'")]
    InvalidArgument(&'static str),
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum Command {
    Call { name: String, data: CallData },
    Result { name: String, data: ResultData },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallData {
    pub id: String,
    pub origin: String,
    pub args: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultData {
    pub id: String,
    pub origin: String,
    pub retval: serde_json::Value,
}

#[derive(FromArgs)]
/// An everest Node.
struct Args {
    /// prefix of installation.
    #[argh(option)]
    #[allow(unused)]
    pub prefix: PathBuf,

    /// configuration yml that we are running.
    #[argh(option)]
    #[allow(unused)]
    pub conf: PathBuf,

    /// module name for us.
    #[argh(option)]
    pub module: String,
}

// TODO(sirver): A lot of this should probably be in something like "internal".
pub fn initialize_mqtt(module: &str) -> (AsyncClient, EventLoop, String) {
    let args: Args = argh::from_env();

    // TODO(sirver): This should probably not be hardcoded, but I have no idea how Everest
    // distributes this knowledge.
    let mqtt_socket: SocketAddr = "127.0.0.1:1883".parse().unwrap();

    // Setup the mqtt client.
    let mut mqtt_options = MqttOptions::new(
        format!("{}/{}", module, args.module),
        mqtt_socket.ip().to_string(),
        mqtt_socket.port(),
    );
    mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));

    let (client, event_loop) = AsyncClient::new(mqtt_options, 10);
    (client, event_loop, args.module)
}
