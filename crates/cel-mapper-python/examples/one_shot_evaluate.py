#!/usr/bin/env python3
"""
One-shot evaluation: compile YAML on every call (fine for scripts and REPL).

Prerequisite: ``maturin develop`` in ``crates/cel-mapper-python``, then::

    python examples/one_shot_evaluate.py
"""

from __future__ import annotations

import pprint

from cel_mapper import MappingRuntime


MAPPING_YAML = """
version: "0.1"
name: demo_sum
records:
  math:
    fields:
      total: "type_int(source.a) + type_int(source.b)"
"""


def main() -> None:
    rt = MappingRuntime()
    source = {"a": 21, "b": 21}
    # Optional evaluation context (merged into ctx for CEL); empty dict if unused.
    context = {"timezone": "UTC"}
    out = rt.evaluate(MAPPING_YAML, source, context)
    pprint.pp(out)


if __name__ == "__main__":
    main()
