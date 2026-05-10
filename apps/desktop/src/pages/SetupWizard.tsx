import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '../stores/appStore';

interface DiscoveredHub {
  hub_name: string;
  hub_id: string;
  hub_url: string;
  mcp_url: string;
  address: string;
}

export function SetupWizard() {
  const navigate = useNavigate();
  const { setMode, setHubUrl, setConfigured } = useAppStore();
  const [step, setStep] = useState(1);
  const [selectedMode, setSelectedMode] = useState<'standalone' | 'hub' | 'client'>('standalone');
  const [hubUrlInput, setHubUrlInput] = useState('');
  const [discoveredHubs, setDiscoveredHubs] = useState<DiscoveredHub[]>([]);
  const [scanning, setScanning] = useState(false);
  const [selectedHub, setSelectedHub] = useState<DiscoveredHub | null>(null);

  useEffect(() => {
    if (step === 2 && selectedMode === 'client') {
      scanForHubs();
    }
  }, [step, selectedMode]);

  const scanForHubs = async () => {
    setScanning(true);
    setDiscoveredHubs([]);
    
    try {
      // 尝试访问常见IP的discovery端点
      const commonIps = [
        '192.168.1.100',
        '192.168.1.101',
        '192.168.0.100',
        '192.168.0.101',
        '10.0.0.1',
        '10.0.0.100',
      ];
      
      const hubs: DiscoveredHub[] = [];
      
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
            });
          }
        } catch (e) {
          // 忽略
        }
      });
      
      await Promise.all(promises);
      setDiscoveredHubs(hubs);
    } catch (error) {
      console.error('Scan failed:', error);
    } finally {
      setScanning(false);
    }
  };

  const handleNext = () => {
    if (step === 1) {
      setStep(2);
    } else {
      handleFinish();
    }
  };

  const handleFinish = () => {
    setMode(selectedMode);
    
    if (selectedMode === 'client') {
      const url = selectedHub ? selectedHub.hub_url : hubUrlInput;
      if (url) {
        setHubUrl(url);
      }
    }
    
    setConfigured(true);
    navigate('/');
  };

  const handleSelectHub = (hub: DiscoveredHub) => {
    setSelectedHub(hub);
    setHubUrlInput(hub.hub_url);
  };

  return (
    <div className="min-h-screen bg-gray-100 flex items-center justify-center">
      <div className="bg-white rounded-lg shadow-lg p-8 w-full max-w-2xl">
        <h1 className="text-2xl font-bold mb-6 text-center">K-HUB Desktop</h1>
        
        {/* 步骤指示器 */}
        <div className="flex justify-center mb-8">
          <div className="flex items-center">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
              step >= 1 ? 'bg-blue-500 text-white' : 'bg-gray-200 text-gray-500'
            }`}>
              1
            </div>
            <div className={`w-16 h-1 ${step >= 2 ? 'bg-blue-500' : 'bg-gray-200'}`} />
            <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
              step >= 2 ? 'bg-blue-500 text-white' : 'bg-gray-200 text-gray-500'
            }`}>
              2
            </div>
          </div>
        </div>
        
        {/* 步骤1: 选择模式 */}
        {step === 1 && (
          <div>
            <h2 className="text-lg font-semibold mb-4 text-center">选择运行模式</h2>
            <div className="grid grid-cols-3 gap-4 mb-6">
              <button
                onClick={() => setSelectedMode('standalone')}
                className={`p-6 rounded-lg border-2 text-center transition-all ${
                  selectedMode === 'standalone'
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="text-4xl mb-3">💻</div>
                <div className="font-semibold mb-1">单机模式</div>
                <div className="text-sm text-gray-500">
                  所有服务本地运行
                </div>
                <div className="text-xs text-gray-400 mt-2">SQLite数据库</div>
              </button>
              
              <button
                onClick={() => setSelectedMode('hub')}
                className={`p-6 rounded-lg border-2 text-center transition-all ${
                  selectedMode === 'hub'
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="text-4xl mb-3">🖥️</div>
                <div className="font-semibold mb-1">Hub模式</div>
                <div className="text-sm text-gray-500">
                  作为中心节点
                </div>
                <div className="text-xs text-gray-400 mt-2">PostgreSQL数据库</div>
              </button>
              
              <button
                onClick={() => setSelectedMode('client')}
                className={`p-6 rounded-lg border-2 text-center transition-all ${
                  selectedMode === 'client'
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="text-4xl mb-3">📱</div>
                <div className="font-semibold mb-1">Client模式</div>
                <div className="text-sm text-gray-500">
                  连接到Hub
                </div>
                <div className="text-xs text-gray-400 mt-2">本地缓存</div>
              </button>
            </div>
          </div>
        )}
        
        {/* 步骤2: 配置 */}
        {step === 2 && (
          <div>
            <h2 className="text-lg font-semibold mb-4 text-center">配置</h2>
            
            {selectedMode === 'standalone' && (
              <div className="bg-gray-50 rounded-lg p-6 mb-6">
                <h3 className="font-semibold mb-2">单机模式</h3>
                <p className="text-gray-600 mb-4">
                  所有服务将在本地运行，包括：
                </p>
                <ul className="list-disc list-inside text-gray-600 space-y-1">
                  <li>Hub Service (API服务)</li>
                  <li>SQLite数据库</li>
                  <li>Collector (文件监听)</li>
                  <li>MCP Server (Agent接口)</li>
                </ul>
                <p className="text-sm text-gray-500 mt-4">
                  适合个人使用，无需安装额外数据库。
                </p>
              </div>
            )}
            
            {selectedMode === 'hub' && (
              <div className="bg-gray-50 rounded-lg p-6 mb-6">
                <h3 className="font-semibold mb-2">Hub模式</h3>
                <p className="text-gray-600 mb-4">
                  作为中心节点运行，其他设备可连接：
                </p>
                <ul className="list-disc list-inside text-gray-600 space-y-1">
                  <li>PostgreSQL数据库</li>
                  <li>完整Hub Service</li>
                  <li>同步服务</li>
                  <li>设备管理</li>
                </ul>
                <p className="text-sm text-gray-500 mt-4">
                  适合作为家庭/团队的知识中心。
                </p>
              </div>
            )}
            
            {selectedMode === 'client' && (
              <div className="bg-gray-50 rounded-lg p-6 mb-6">
                <h3 className="font-semibold mb-2">Client模式</h3>
                <p className="text-gray-600 mb-4">
                  连接到Hub，同步数据：
                </p>
                
                {/* 自动发现的Hub */}
                <div className="mb-4">
                  <div className="flex items-center justify-between mb-2">
                    <label className="text-sm font-medium text-gray-700">
                      发现的Hub
                    </label>
                    <button
                      onClick={scanForHubs}
                      disabled={scanning}
                      className="text-sm text-blue-500 hover:text-blue-700"
                    >
                      {scanning ? '扫描中...' : '重新扫描'}
                    </button>
                  </div>
                  
                  {discoveredHubs.length > 0 ? (
                    <div className="space-y-2">
                      {discoveredHubs.map((hub) => (
                        <div
                          key={hub.hub_id}
                          onClick={() => handleSelectHub(hub)}
                          className={`p-3 rounded-lg border cursor-pointer transition-all ${
                            selectedHub?.hub_id === hub.hub_id
                              ? 'border-blue-500 bg-blue-50'
                              : 'border-gray-200 hover:border-gray-300'
                          }`}
                        >
                          <div className="font-medium">{hub.hub_name}</div>
                          <div className="text-sm text-gray-500">{hub.hub_url}</div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm text-gray-500">
                      {scanning ? '正在扫描...' : '未发现Hub，请手动输入'}
                    </p>
                  )}
                </div>
                
                {/* 手动输入 */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Hub地址
                  </label>
                  <input
                    type="text"
                    value={hubUrlInput}
                    onChange={(e) => {
                      setHubUrlInput(e.target.value);
                      setSelectedHub(null);
                    }}
                    placeholder="http://192.168.1.100:8443"
                    className="w-full px-4 py-2 border rounded-lg"
                  />
                </div>
              </div>
            )}
          </div>
        )}
        
        {/* 按钮 */}
        <div className="flex justify-end gap-2">
          {step > 1 && (
            <button
              onClick={() => setStep(step - 1)}
              className="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              上一步
            </button>
          )}
          <button
            onClick={handleNext}
            disabled={selectedMode === 'client' && step === 2 && !hubUrlInput}
            className="px-6 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {step === 1 ? '下一步' : '完成'}
          </button>
        </div>
      </div>
    </div>
  );
}
