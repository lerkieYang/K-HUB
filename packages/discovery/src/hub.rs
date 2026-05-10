use crate::{DiscoveryMessage, DISCOVERY_PORT};
use tokio::net::UdpSocket;
use std::net::SocketAddr;

/// Hub端发现服务
pub struct HubDiscovery {
    hub_name: String,
    hub_id: String,
    hub_url: String,
    mcp_url: String,
}

impl HubDiscovery {
    pub fn new(hub_name: &str, hub_id: &str, hub_url: &str, mcp_url: &str) -> Self {
        Self {
            hub_name: hub_name.to_string(),
            hub_id: hub_id.to_string(),
            hub_url: hub_url.to_string(),
            mcp_url: mcp_url.to_string(),
        }
    }
    
    /// 启动发现服务
    pub async fn start(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)).await?;
        socket.set_broadcast(true)?;
        
        tracing::info!("Hub discovery service started on port {}", DISCOVERY_PORT);
        
        let mut buf = vec![0u8; 1024];
        
        loop {
            let (len, addr) = socket.recv_from(&mut buf).await?;
            
            if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..len]) {
                match msg {
                    DiscoveryMessage::Discover { device_name, device_id } => {
                        tracing::info!("Discovery request from {} ({})", device_name, device_id);
                        
                        // 发送响应
                        let response = DiscoveryMessage::HubAnnounce {
                            hub_name: self.hub_name.clone(),
                            hub_id: self.hub_id.clone(),
                            hub_url: self.hub_url.clone(),
                            mcp_url: self.mcp_url.clone(),
                            requires_invite: true,
                        };
                        
                        let response_bytes = serde_json::to_vec(&response)?;
                        socket.send_to(&response_bytes, addr).await?;
                    }
                    _ => {}
                }
            }
        }
    }
    
    /// 启动定期广播
    pub async fn start_beacon(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        let beacon = DiscoveryMessage::HubBeacon {
            hub_name: self.hub_name.clone(),
            hub_id: self.hub_id.clone(),
            hub_url: self.hub_url.clone(),
            mcp_url: self.mcp_url.clone(),
        };
        
        let beacon_bytes = serde_json::to_vec(&beacon)?;
        let broadcast_addr: SocketAddr = crate::BROADCAST_ADDR.parse()?;
        
        tracing::info!("Starting hub beacon");
        
        loop {
            socket.send_to(&beacon_bytes, broadcast_addr).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}
