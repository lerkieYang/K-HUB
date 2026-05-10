import os

# 检查 WSL 路径是否可访问
hermes_memory = r"\\wsl.localhost\Ubuntu\home\lerekie\.hermes\memories"
print(f"Checking: {hermes_memory}")
print(f"Exists: {os.path.exists(hermes_memory)}")

if os.path.exists(hermes_memory):
    print(f"Files: {os.listdir(hermes_memory)}")
    
    memory_file = os.path.join(hermes_memory, "MEMORY.md")
    if os.path.exists(memory_file):
        with open(memory_file, 'r', encoding='utf-8') as f:
            content = f.read()
            print(f"\nMEMORY.md size: {len(content)} bytes")
            print(f"First 200 chars: {content[:200]}")
