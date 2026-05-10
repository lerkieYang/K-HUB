import sqlite3
import os

# Check database from hub-service perspective
db_path = './data/knowledge-hub.db'
abs_path = os.path.abspath(db_path)
print(f"Working dir: {os.getcwd()}")
print(f"DB path: {db_path}")
print(f"Absolute: {abs_path}")
print(f"Exists: {os.path.exists(db_path)}")

if os.path.exists(db_path):
    conn = sqlite3.connect(db_path)
    c = conn.cursor()
    c.execute('SELECT COUNT(*) FROM memory')
    total = c.fetchone()[0]
    print(f"Total memories: {total}")
    
    c.execute('SELECT source_type, COUNT(*) FROM memory GROUP BY source_type')
    for r in c.fetchall():
        print(f"  {r[0]}: {r[1]}")
    
    conn.close()
