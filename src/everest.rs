use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("mqtt error")]
    Mqtt(#[from] rumqttc::ClientError),
    #[error("missing argument to command call: '{0}'")]
    MissingArgument(&'static str),
}

pub type Result<T> = ::std::result::Result<T, Error>;
