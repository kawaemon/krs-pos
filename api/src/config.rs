use serde::Deserialize;
use std::net::SocketAddrV4;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub listen_addr: SocketAddrV4,
    pub db_uri: String,
}
