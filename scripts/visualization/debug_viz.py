
from pathlib import Path

HTML_FILE = Path("site/spec-evolution") / "visualization_of_the_evolution_of_the_frankensqlite_specs_document_from_inception.html"

try:
    with HTML_FILE.open("r", encoding="utf-8") as f:
        content = f.read()
    print(f"Read {len(content)} chars from {HTML_FILE}")
except Exception as e:
    print(f"Error: {e}")
