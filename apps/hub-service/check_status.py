import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()
c.execute('SELECT status, COUNT(*) FROM memory WHERE source_type = "knowledge" GROUP BY status')
for r in c.fetchall():
    print(f'  status={r[0]}: {r[1]}')
c.execute('SELECT COUNT(*) FROM memory WHERE source_type = "knowledge" AND status = "active"')
print('active knowledge:', c.fetchone()[0])
c.execute('SELECT source_type, status, COUNT(*) FROM memory GROUP BY source_type, status')
for r in c.fetchall():
    print(f'  {r[0]} status={r[1]}: {r[2]}')
conn.close()
