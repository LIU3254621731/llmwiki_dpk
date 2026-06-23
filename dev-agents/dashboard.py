"""
Dashboard Server — 轻量HTTP服务器，实时展示Agent开发进度
运行方式: python dev-agents/dashboard.py
然后浏览器打开 http://localhost:9999
"""
import json
import http.server
import os
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASK_FILE = HERE / "task_registry.json"
PORT = 9999


class DashboardHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(HERE / "dashboard"), **kwargs)
    
    def do_GET(self):
        if self.path == "/api/tasks":
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            with open(TASK_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
            self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))
        else:
            super().do_GET()
    
    def log_message(self, format, *args):
        pass  # 静默日志


def main():
    print(f"""
╔══════════════════════════════════════════════╗
║   🤖 LLMWiki Dev-Agents Dashboard           ║
║                                              ║
║   Dashboard:  http://localhost:{PORT}          ║
║   API:        http://localhost:{PORT}/api/tasks ║
║                                              ║
║   Press Ctrl+C to stop                       ║
╚══════════════════════════════════════════════╝
""")
    server = http.server.HTTPServer(("0.0.0.0", PORT), DashboardHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nDashboard stopped.")
        server.shutdown()


if __name__ == "__main__":
    main()
