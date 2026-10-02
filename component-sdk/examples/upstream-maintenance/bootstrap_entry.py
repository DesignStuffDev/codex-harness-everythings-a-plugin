#!/usr/bin/env python3
"""Read-only impact inspection even when the Codex host cannot start."""

from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().with_name("plugin.pyz")))
from maintenance_review import main

if __name__ == "__main__":
    raise SystemExit(main())
