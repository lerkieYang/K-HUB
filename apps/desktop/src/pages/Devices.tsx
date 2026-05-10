import { useToast } from '../components/Toast';
import React, { useState, useEffect } from 'react';
import { useAppStore, getBackendUrl } from '../stores/appStore';

interface Device {
  id: string;
  name: string;
  role: string;
  status: string;
  os?: string;
  last_seen_at?: string;
}

export function Devices() {
  const { hubUrl } = useAppStore();
  const toast = useToast();
  const [devices, setDevices] = useState<Device[]>([]);
  const [showInvite, setShowInvite] = useState(false);
  const [inviteCode, setInviteCode] = useState('');

  useEffect(() => {
    fetchDevices();
  }, []);

  const fetchDevices = async () => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/devices`);
      const data = await res.json();
      setDevices(data.devices || []);
    } catch (error) {
      console.error('Failed to fetch devices:', error);
    }
  };

  const handleCreateInvite = async () => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/invites`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ttl_seconds: 600 }),
      });
      const data = await res.json();
      setInviteCode(data.invite_code);
      setShowInvite(true);
    } catch (error) {
      console.error('Failed to create invite:', error);
    }
  };

  const handleApprove = async (deviceId: string) => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/devices/${deviceId}/approve`, { method: 'POST' });
      fetchDevices();
    } catch (error) {
      console.error('Failed to approve device:', error);
    }
  };

  const handleRevoke = async (deviceId: string) => {
    if (!await toast.confirm('确定撤销此设备？', '确认', { danger: true })) return;
    
    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/devices/${deviceId}`, { method: 'DELETE' });
      fetchDevices();
    } catch (error) {
      console.error('Failed to revoke device:', error);
    }
  };

  const copyInviteCode = () => {
    navigator.clipboard.writeText(inviteCode);
    alert('邀请码已复制到剪贴板');
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online': return 'bg-green-100 text-green-800';
      case 'approved': return 'bg-blue-100 text-blue-800';
      case 'pending': return 'bg-yellow-100 text-yellow-800';
      case 'offline': return 'bg-gray-100 text-gray-800';
      case 'revoked': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">设备管理</h1>
        <button
          onClick={handleCreateInvite}
          className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          生成邀请码
        </button>
      </div>
      
      {/* 设备列表 */}
      <div className="bg-white rounded-lg shadow">
        {devices.length === 0 ? (
          <div className="p-8 text-center text-gray-500">
            暂无设备，点击"生成邀请码"添加新设备
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">设备名</th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">角色</th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">状态</th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">操作系统</th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">最后心跳</th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">操作</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200">
              {devices.map((device) => (
                <tr key={device.id}>
                  <td className="px-4 py-3 text-sm font-medium">{device.name}</td>
                  <td className="px-4 py-3 text-sm">
                    <span className="px-2 py-1 bg-purple-100 text-purple-800 rounded text-xs">
                      {device.role}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-sm">
                    <span className={`px-2 py-1 rounded text-xs ${getStatusColor(device.status)}`}>
                      {device.status}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-500">{device.os || '-'}</td>
                  <td className="px-4 py-3 text-sm text-gray-500">
                    {device.last_seen_at
                      ? new Date(device.last_seen_at).toLocaleString()
                      : '-'}
                  </td>
                  <td className="px-4 py-3 text-sm">
                    {device.status === 'pending' && (
                      <button
                        onClick={() => handleApprove(device.id)}
                        className="text-green-500 hover:text-green-700 mr-3"
                      >
                        批准
                      </button>
                    )}
                    {device.status !== 'revoked' && (
                      <button
                        onClick={() => handleRevoke(device.id)}
                        className="text-red-500 hover:text-red-700"
                      >
                        撤销
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      
      {/* 邀请码弹窗 */}
      {showInvite && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h2 className="text-xl font-bold mb-4">设备邀请码</h2>
            
            <div className="bg-gray-100 rounded-lg p-4 mb-4">
              <p className="text-sm text-gray-500 mb-2">邀请码（10分钟有效）：</p>
              <p className="font-mono text-lg break-all">{inviteCode}</p>
            </div>
            
            <p className="text-sm text-gray-500 mb-4">
              在新设备的K-HUB Desktop中输入此邀请码即可加入。
            </p>
            
            <div className="flex justify-end gap-2">
              <button
                onClick={copyInviteCode}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                复制邀请码
              </button>
              <button
                onClick={() => setShowInvite(false)}
                className="px-4 py-2 text-gray-600 hover:text-gray-800"
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
