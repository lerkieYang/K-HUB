import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()
c.execute("DELETE FROM memory WHERE source_type = 'knowledge'")
print(f"Deleted {c.rowcount} knowledge records")
conn.commit()
conn.close()
