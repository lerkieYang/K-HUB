import urllib.request
import json

data = json.dumps({
    "name": "Google Drive",
    "paths": ["U:\\google drive"],
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
