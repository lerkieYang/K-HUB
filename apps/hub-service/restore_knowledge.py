import sqlite3
conn = sqlite3.connect(r'C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db')
c = conn.cursor()
c.execute("UPDATE memory SET status = 'active' WHERE source_type = 'knowledge' AND status = 'deleted'")
print(f"Restored {c.rowcount} knowledge records to active")
conn.commit()
conn.close()
