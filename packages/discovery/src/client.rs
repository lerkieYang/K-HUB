use crate::{DiscoveryMessage, DiscoveredHub, DISCOVERY_PORT};
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Client端发现服务
pub struct ClientDiscovery {
    device_name: String,
    device_id: String,
    discovered_hubs: Arc<Mutex<HashMap<String, DiscoveredHub>>>,
}

impl ClientDiscovery {
    pub fn new(device_name: &str, device_id: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            device_id: device_id.to_string(),
            discovered_hubs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 发送发现请求并收集响应
    pub async fn discover(&self, timeout_secs: u64) -> anyhow::Result<Vec<DiscoveredHub>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        let discover_msg = DiscoveryMessage::Discover {
            device_name: self.device_name.clone(),
            device_id: self.device_id.clone(),
        };
        
        let msg_bytes = serde_json::to_vec(&discover_msg)?;
        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", DISCOVERY_PORT).parse()?;
        
        // 发送发现请求
        tracing::info!("Sending discovery request...");
        socket.send_to(&msg_bytes, broadcast_addr).await?;
        
        // 等待响应
        let discovered = self.collect_responses(&socket, timeout_secs).await?;
        
        tracing::info!("Discovered {} hubs", discovered.len());
        
        Ok(discovered)
    }
    
    /// 收集Hub响应
    async fn collect_responses(&self, socket: &UdpSocket, timeout_secs: u64) -> anyhow::Result<Vec<DiscoveredHub>> {
        let mut hubs = Vec::new();
        let mut buf = vec![0u8; 1024];
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);
        
        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            
            let remaining = deadline - tokio::time::Instant::now();
            
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..len]) {
                        match msg {
                            DiscoveryMessage::HubAnnounce { hub_name, hub_id, hub_url, mcp_url, requires_invite } => {
                                let hub = DiscoveredHub {
                                    hub_name,
                                    hub_id,
                                    hub_url,
                                    mcp_url,
                                    address: addr,
                                    discovered_at: chrono::Utc::now(),
                                    requires_invite,
                                };
                                
                                // 去重
                                if !hubs.iter().any(|h: &DiscoveredHub| h.hub_id == hub.hub_id) {
                                    tracing::info!("Found hub: {} at {}", hub.hub_name, hub.hub_url);
                                    hubs.push(hub);
                                }
                            }
                            DiscoveryMessage::HubBeacon { hub_name, hub_id, hub_url, mcp_url } => {
                                let hub = DiscoveredHub {
                                    hub_name,
                                    hub_id,
                                    hub_url,
                                    mcp_url,
                                    address: addr,
                                    discovered_at: chrono::Utc::now(),
                                    requires_invite: true,
                                };
                                
                                if !hubs.iter().any(|h: &DiscoveredHub| h.hub_id == hub.hub_id) {
                                    tracing::info!("Found hub via beacon: {} at {}", hub.hub_name, hub.hub_url);
                                    hubs.push(hub);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Receive error: {}", e);
                }
                Err(_) => {
                    // 超时
                    break;
                }
            }
        }
        
        Ok(hubs)
    }
    
    /// 监听Hub广播
    pub async fn listen_beacons(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)).await?;
        socket.set_broadcast(true)?;
        
        tracing::info!("Listening for hub beacons...");
        
        let mut buf = vec![0u8; 1024];
        
        loop {
            let (len, addr) = socket.recv_from(&mut buf).await?;
            
            if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..len]) {
                if let DiscoveryMessage::HubBeacon { hub_name, hub_id, hub_url, mcp_url } = msg {
                    let hub = DiscoveredHub {
                        hub_name,
                        hub_id,
                        hub_url,
                        mcp_url,
                        address: addr,
                        discovered_at: chrono::Utc::now(),
                        requires_invite: true,
                    };
                    
                    // 更新发现的Hub列表
                    if let Ok(mut hubs) = self.discovered_hubs.lock() {
                        hubs.insert(hub.hub_id.clone(), hub);
                    }
                }
            }
        }
    }
    
    /// 获取已发现的Hub列表
    pub fn get_discovered_hubs(&self) -> Vec<DiscoveredHub> {
        if let Ok(hubs) = self.discovered_hubs.lock() {
            hubs.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}
