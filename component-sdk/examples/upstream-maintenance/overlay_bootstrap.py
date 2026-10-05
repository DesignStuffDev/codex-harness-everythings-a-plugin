#!/usr/bin/env python3
"""Inspect a retained candidate overlay and source capsule without host or Git."""

import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().with_name("plugin.pyz")))
from candidate_overlay import inspect_overlay

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("job", type=Path)
    result = inspect_overlay(parser.parse_args().job)
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(0 if result["status"] == "prepared" else 2)
