import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()
c.execute('SELECT COUNT(*) FROM knowledge_base_config')
print('configs:', c.fetchone()[0])
c.execute('SELECT COUNT(*) FROM memory')
print('memories:', c.fetchone()[0])
c.execute('SELECT source_type, COUNT(*) FROM memory GROUP BY source_type')
for r in c.fetchall():
    print(f'  {r[0]}: {r[1]}')
c.execute('SELECT COUNT(*) FROM memory WHERE source_type = "knowledge"')
print('knowledge records:', c.fetchone()[0])
conn.close()
