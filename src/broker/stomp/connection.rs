#[derive(Clone, Copy)]
pub struct HeartBeat(pub u32, pub u32);
#[derive(Clone, Copy)]
pub struct Credentials<'a>(pub &'a str, pub &'a str);
#[derive(Clone)]
pub struct OwnedCredentials {
    pub login: String,
    pub passcode: String,
}

impl OwnedCredentials {
    pub fn from<'a>(credentials: Credentials<'a>) -> OwnedCredentials {
        OwnedCredentials {
            login: credentials.0.to_owned(),
            passcode: credentials.1.to_owned(),
        }
    }
}

fn heartbeat(client_ms: u32, server_ms: u32) -> u32 {
    if client_ms == 0 || server_ms == 0 {
        0
    } else {
        client_ms.max(server_ms)
    }
}

pub fn select_heartbeat(
    client_tx_ms: u32,
    client_rx_ms: u32,
    server_tx_ms: u32,
    server_rx_ms: u32,
) -> (u32, u32) {
    (
        heartbeat(client_tx_ms, server_tx_ms),
        heartbeat(client_rx_ms, server_rx_ms),
    )
}
