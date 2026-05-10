import sqlite3
import sys

# Check the database
db_path = r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db'
conn = sqlite3.connect(db_path)
c = conn.cursor()

print(f"Database: {db_path}")

# Check all tables
c.execute("SELECT name FROM sqlite_master WHERE type='table'")
tables = [r[0] for r in c.fetchall()]
print(f"Tables: {tables}")

# Check memory table
c.execute('SELECT COUNT(*) FROM memory')
print(f"Total memories: {c.fetchone()[0]}")

c.execute('SELECT source_type, COUNT(*) FROM memory GROUP BY source_type')
for r in c.fetchall():
    print(f"  {r[0]}: {r[1]}")

# Check knowledge specifically
c.execute('SELECT COUNT(*) FROM memory WHERE source_type = "knowledge"')
print(f"Knowledge records: {c.fetchone()[0]}")

# Check a sample knowledge record
c.execute('SELECT id, title, tags FROM memory WHERE source_type = "knowledge" LIMIT 3')
for r in c.fetchall():
    print(f"  Sample: {r[0][:8]}... title={r[1][:40]} tags={r[2][:50] if r[2] else 'None'}")

conn.close()
