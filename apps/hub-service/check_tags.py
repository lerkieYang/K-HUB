import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()

# Check knowledge records and their tags
c.execute("SELECT COUNT(*) FROM memory WHERE source_type = 'knowledge' AND status = 'active'")
print(f"Active knowledge records: {c.fetchone()[0]}")

c.execute("SELECT COUNT(*) FROM memory WHERE source_type = 'knowledge' AND status = 'deleted'")
print(f"Deleted knowledge records: {c.fetchone()[0]}")

# Check all records with empty tags
c.execute("SELECT source_type, COUNT(*) FROM memory WHERE status = 'active' AND (tags IS NULL OR tags = '[]' OR tags = '') GROUP BY source_type")
print("\nRecords with empty tags:")
for r in c.fetchall():
    print(f"  {r[0]}: {r[1]}")

# Check all records
c.execute("SELECT source_type, COUNT(*) FROM memory WHERE status = 'active' GROUP BY source_type")
print("\nAll active records:")
for r in c.fetchall():
    print(f"  {r[0]}: {r[1]}")

conn.close()
