import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()

# Check source_file_path patterns
c.execute("SELECT source_file_path, COUNT(*) FROM memory WHERE source_type = 'knowledge' AND status = 'active' GROUP BY source_file_path LIMIT 20")
for r in c.fetchall():
    path = r[0] if r[0] else 'NULL'
    print(f"  {r[1]}: {path[:100]}")

# Check by directory pattern
c.execute("""
    SELECT 
        CASE 
            WHEN source_file_path LIKE '%hermes%' THEN 'hermes'
            WHEN source_file_path LIKE '%openclaw%' THEN 'openclaw'
            WHEN source_file_path LIKE '%WPS%' THEN 'WPS云盘'
            WHEN source_file_path LIKE '%google drive%' THEN 'Google Drive'
            WHEN source_file_path LIKE '%codex%' THEN 'codex'
            ELSE 'other'
        END as source,
        COUNT(*) 
    FROM memory 
    WHERE source_type = 'knowledge' AND status = 'active'
    GROUP BY source
""")
print("\nBy directory:")
for r in c.fetchall():
    print(f"  {r[0]}: {r[1]}")

conn.close()
