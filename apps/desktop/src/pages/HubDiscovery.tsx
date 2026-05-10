import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '../stores/appStore';

interface DiscoveredHub {
  hub_name: string;
  hub_id: string;
  hub_url: string;
  mcp_url: string;
  address: string;
  discovered_at: string;
  requires_invite: boolean;
}

export function HubDiscovery() {
  const navigate = useNavigate();
  const { setMode, setHubUrl, setConfigured } = useAppStore();
  const [hubs, setHubs] = useState<DiscoveredHub[]>([]);
  const [scanning, setScanning] = useState(false);
  const [manualUrl, setManualUrl] = useState('');
  const [inviteCode, setInviteCode] = useState('');
  const [showInviteModal, setShowInviteModal] = useState(false);
  const [selectedHub, setSelectedHub] = useState<DiscoveredHub | null>(null);

  useEffect(() => {
    // 自动扫描一次
    scanForHubs();
  }, []);

  const scanForHubs = async () => {
    setScanning(true);
    setHubs([]);
    
    try {
      // 方法1: 尝试局域网广播发现
      const discovered = await discoverViaBroadcast();
      
      // 方法2: 尝试访问 /api/discovery/info 端点
      const localHubs = await discoverViaHttp();
      
      // 合并结果
      const allHubs = [...discovered, ...localHubs];
      const uniqueHubs = allHubs.filter((hub, index, self) =>
        index === self.findIndex(h => h.hub_id === hub.hub_id)
      );
      
      setHubs(uniqueHubs);
    } catch (error) {
      console.error('Discovery failed:', error);
    } finally {
      setScanning(false);
    }
  };

  const discoverViaBroadcast = async (): Promise<DiscoveredHub[]> => {
    // 由于浏览器限制，无法直接发送UDP广播
    // 需要通过Tauri调用Rust后端
    // 这里返回空数组，实际实现需要调用IPC
    return [];
  };

  const discoverViaHttp = async (): Promise<DiscoveredHub[]> => {
    const hubs: DiscoveredHub[] = [];
    
    // 尝试常见IP段
    const commonIps = [
      '192.168.1.100',
      '192.168.1.101',
      '192.168.0.100',
      '192.168.0.101',
      '10.0.0.1',
      '10.0.0.100',
    ];
    
    // 并发尝试
    const promises = commonIps.map(async (ip) => {
      try {
        const response = await fetch(`http://${ip}:8443/api/discovery/info`, {
          signal: AbortSignal.timeout(2000),
        });
        
        if (response.ok) {
          const data = await response.json();
          hubs.push({
            hub_name: data.hub_name,
            hub_id: data.hub_id,
            hub_url: data.hub_url,
            mcp_url: data.mcp_url,
            address: ip,
            discovered_at: new Date().toISOString(),
            requires_invite: data.requires_invite,
          });
        }
      } catch (e) {
        // 忽略连接失败
      }
    });
    
    await Promise.all(promises);
    return hubs;
  };

  const handleSelectHub = (hub: DiscoveredHub) => {
    setSelectedHub(hub);
    setShowInviteModal(true);
  };

  const handleManualConnect = () => {
    if (!manualUrl) return;
    
    setSelectedHub({
      hub_name: 'Manual Hub',
      hub_id: 'manual',
      hub_url: manualUrl,
      mcp_url: `${manualUrl}/mcp`,
      address: manualUrl,
      discovered_at: new Date().toISOString(),
      requires_invite: true,
    });
    setShowInviteModal(true);
  };

  const handleConnect = async () => {
    if (!selectedHub || !inviteCode) {
      alert('请输入邀请码');
      return;
    }
    
    try {
      // 注册设备
      const response = await fetch(`${selectedHub.hub_url}/api/devices/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          invite_code: inviteCode,
          device_name: navigator.userAgent,
          os: navigator.platform,
          app_version: '0.1.0',
        }),
      });
      
      const data = await response.json();
      
      if (data.device_id) {
        // 切换到Client模式
        setMode('client');
        setHubUrl(selectedHub.hub_url);
        setConfigured(true);
        
        alert(`设备注册成功！设备ID: ${data.device_id}\n等待Hub管理员审批...`);
        navigate('/');
      } else {
        alert(`注册失败: ${data.error || 'Unknown error'}`);
      }
    } catch (error) {
      alert(`连接失败: ${error}`);
    }
  };

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">发现Hub设备</h1>
      
      {/* 扫描按钮 */}
      <div className="mb-6">
        <button
          onClick={scanForHubs}
          disabled={scanning}
          className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
        >
          {scanning ? '扫描中...' : '🔍 扫描局域网'}
        </button>
      </div>
      
      {/* 发现的Hub列表 */}
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-4">发现的Hub</h2>
        
        {hubs.length === 0 ? (
          <div className="bg-gray-50 rounded-lg p-8 text-center text-gray-500">
            {scanning ? '正在扫描...' : '未发现Hub设备'}
          </div>
        ) : (
          <div className="space-y-4">
            {hubs.map((hub) => (
              <div
                key={hub.hub_id}
                className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow cursor-pointer"
                onClick={() => handleSelectHub(hub)}
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-semibold">{hub.hub_name}</h3>
                    <p className="text-sm text-gray-500">{hub.hub_url}</p>
                    <p className="text-xs text-gray-400">ID: {hub.hub_id}</p>
                  </div>
                  <button className="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600">
                    连接
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      
      {/* 手动输入 */}
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-lg font-semibold mb-4">手动连接</h2>
        <p className="text-sm text-gray-500 mb-4">
          如果自动发现失败，可以手动输入Hub地址
        </p>
        
        <div className="flex gap-2">
          <input
            type="text"
            value={manualUrl}
            onChange={(e) => setManualUrl(e.target.value)}
            placeholder="http://192.168.1.100:8443"
            className="flex-1 px-4 py-2 border rounded-lg"
          />
          <button
            onClick={handleManualConnect}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            连接
          </button>
        </div>
      </div>
      
      {/* 邀请码弹窗 */}
      {showInviteModal && selectedHub && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h2 className="text-xl font-bold mb-4">连接到 {selectedHub.hub_name}</h2>
            
            <div className="mb-4">
              <p className="text-sm text-gray-500 mb-2">Hub地址：</p>
              <p className="font-mono text-sm">{selectedHub.hub_url}</p>
            </div>
            
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 mb-1">
                邀请码
              </label>
              <input
                type="text"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                placeholder="输入Hub管理员提供的邀请码"
                className="w-full px-4 py-2 border rounded-lg"
              />
              <p className="text-xs text-gray-400 mt-1">
                请向Hub管理员获取邀请码
              </p>
            </div>
            
            <div className="flex justify-end gap-2">
              <button
                onClick={() => {
                  setShowInviteModal(false);
                  setInviteCode('');
                }}
                className="px-4 py-2 text-gray-600 hover:text-gray-800"
              >
                取消
              </button>
              <button
                onClick={handleConnect}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                连接
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
