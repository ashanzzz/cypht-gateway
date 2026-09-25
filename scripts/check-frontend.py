#!/usr/bin/env python3
from pathlib import Path
import re
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
html = (root / "frontend/static/index.html").read_text(encoding="utf-8")
blocks = re.findall(r"<script(?:\s[^>]*)?>(.*?)</script>", html, flags=re.S | re.I)
if not blocks:
    raise SystemExit("frontend check failed: no inline script found")
with tempfile.NamedTemporaryFile("w", suffix=".js", encoding="utf-8", delete=False) as fh:
    fh.write("\n".join(blocks))
    name = fh.name
result = subprocess.run(["node", "--check", name], check=False)
Path(name).unlink(missing_ok=True)
raise SystemExit(result.returncode)
