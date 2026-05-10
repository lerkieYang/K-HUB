pub mod hub;
pub mod client;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// 发现协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessage {
    /// Client发送的发现请求
    Discover {
        device_name: String,
        device_id: String,
    },
    /// Hub响应发现请求
    HubAnnounce {
        hub_name: String,
        hub_id: String,
        hub_url: String,
        mcp_url: String,
        requires_invite: bool,
    },
    /// Hub定期广播
    HubBeacon {
        hub_name: String,
        hub_id: String,
        hub_url: String,
        mcp_url: String,
    },
}

/// 发现的Hub信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHub {
    pub hub_name: String,
    pub hub_id: String,
    pub hub_url: String,
    pub mcp_url: String,
    pub address: SocketAddr,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub requires_invite: bool,
}

/// 广播端口
pub const DISCOVERY_PORT: u16 = 5353;

/// 广播地址
pub const BROADCAST_ADDR: &str = "255.255.255.255:5353";
