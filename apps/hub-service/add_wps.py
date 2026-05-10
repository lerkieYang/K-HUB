import urllib.request
import json

data = json.dumps({
    "name": "WPS云盘",
    "paths": ["D:\\WPS云盘\\13822359\\WPS云盘\\工作-weee!"],
    "file_patterns": ["*"],
    "auto_index": True
}).encode('utf-8')

req = urllib.request.Request(
    'http://127.0.0.1:8443/api/knowledge/config',
    data=data,
    headers={'Content-Type': 'application/json'}
)
resp = urllib.request.urlopen(req)
print(resp.read().decode())
