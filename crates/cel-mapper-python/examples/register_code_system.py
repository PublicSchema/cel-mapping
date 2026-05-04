#!/usr/bin/env python3
"""
Register a code system from a Python dict, then use it from a mapping.

Prerequisite: ``maturin develop`` in ``crates/cel-mapper-python``, then::

    python examples/register_code_system.py
"""

from __future__ import annotations

import pprint

from cel_mapper import MappingRuntime


# Same shape as ``code_systems`` entries in mapping YAML: short-id -> entry
GENDER_CODES: dict[str, object] = {
    "m": {
        "id": "canonical.gender.male",
        "label": {"en": "Male"},
        "aliases": ["male", "man"],
    },
    "f": {
        "id": "canonical.gender.female",
        "label": {"en": "Female"},
        "aliases": ["female", "woman"],
    },
}


MAPPING_YAML = """
version: "0.1"
name: demo_codes
records:
  people:
    fields:
      gender_id: "code.map_or_default('demo.gender', type_string(source.raw_gender), 'canonical.gender.unknown')"
"""


def main() -> None:
    rt = MappingRuntime()
    rt.register_code_system("demo.gender", GENDER_CODES)

    compiled = rt.compile_mapping(MAPPING_YAML)
    out = rt.evaluate_compiled(compiled, {"raw_gender": "m"}, {})
    pprint.pp(out)


if __name__ == "__main__":
    main()
