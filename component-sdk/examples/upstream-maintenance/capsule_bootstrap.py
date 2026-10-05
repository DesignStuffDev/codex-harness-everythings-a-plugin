#!/usr/bin/env python3
"""Inspect a sparse source-input capsule, without Git or a running Codex host."""

import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().with_name("plugin.pyz")))
from source_capsule import inspect_capsule

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("job", type=Path)
    result = inspect_capsule(parser.parse_args().job)
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(0 if result["status"] == "sealed" else 2)
