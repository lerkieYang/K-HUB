import urllib.request, json

url = 'http://127.0.0.1:8443/api/ai/test'
data = json.dumps({'provider': 'deepseek', 'api_key': 'sk-test'}).encode()
req = urllib.request.Request(url, data=data, headers={'Content-Type': 'application/json'})
try:
    resp = urllib.request.urlopen(req)
    print(resp.read().decode())
except urllib.error.HTTPError as e:
    print(f'HTTP {e.code}: {e.read().decode()[:200]}')
except Exception as e:
    print(f'Error: {e}')
