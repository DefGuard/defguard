use crate::db::Id;

// Used by the proxy manager to control proxies (start/shutdown).
pub enum ProxyControlMessage {
    StartConnection(Id),
    ShutdownConnection(Id),
}
