#!/usr/bin/env python3
"""
Compile a mapping once, then evaluate many source documents (typical batch / ETL pattern).

Prerequisite: from ``crates/cel-mapper-python`` run ``maturin develop``, then::

    python examples/compile_once_evaluate_many.py
"""

from __future__ import annotations

import pprint

from cel_mapper import MappingRuntime


MAPPING_YAML = """
version: "0.1"
name: demo_greeting
records:
  rows:
    fields:
      id: "type_string(source.id)"
      greeting: '"Hello, " + type_string(source.name) + "!"'
"""


def main() -> None:
    rt = MappingRuntime()
    compiled = rt.compile_mapping(MAPPING_YAML)
    print(f"compiled mapping: {compiled.name!r} v{compiled.version!r}\n")

    sources = [
        {"id": "1", "name": "Ada"},
        {"id": "2", "name": "Grace"},
    ]
    for src in sources:
        # Top-level JSON becomes the CEL binding ``source`` (not wrapped under a ``source`` key).
        out = rt.evaluate_compiled(compiled, src, {})
        print(f"--- source = {src!r}")
        pprint.pp(out)


if __name__ == "__main__":
    main()
