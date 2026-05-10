import urllib.request, json

def test_api(name, url, api_key, data):
    req = urllib.request.Request(url, data=json.dumps(data).encode(), headers={
        'Content-Type': 'application/json',
        'Authorization': f'Bearer {api_key}'
    })
    try:
        with urllib.request.urlopen(req) as resp:
            result = json.loads(resp.read().decode())
            print(f"{name}: SUCCESS")
            return True
    except urllib.error.HTTPError as e:
        body = e.read().decode()[:200]
        print(f"{name}: HTTP {e.code} - {body}")
        return False
    except Exception as e:
        print(f"{name}: Error - {e}")
        return False

# Test DeepSeek
print("=== DeepSeek ===")
test_api(
    "DeepSeek Chat",
    "https://api.deepseek.com/v1/chat/completions",
    "sk-test",
    {"model": "deepseek-chat", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 5}
)

test_api(
    "DeepSeek Embedding",
    "https://api.deepseek.com/v1/embeddings",
    "sk-test",
    {"model": "text-embedding-v1", "input": ["test"]}
)

# Test MiMo
print("\n=== MiMo ===")
test_api(
    "MiMo Chat",
    "https://token-plan-sgp.xiaomimimo.com/v1/chat/completions",
    "tp-s0xs4s1frnckgb1845ngji5zt00mqs2qylclz4m8i6drmyow",
    {"model": "mimo-v2.5-pro", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 5}
)
