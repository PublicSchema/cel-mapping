#!/usr/bin/env python3
"""
Evaluate a single CEL expression (mapping stdlib) with no mapping YAML.

Prerequisite: ``maturin develop`` in ``crates/cel-mapper-python``, then::

    python examples/evaluate_expression_only.py
"""

from __future__ import annotations

import pprint

from cel_mapper import MappingRuntime


def main() -> None:
    rt = MappingRuntime()
    rt.register_code_system(
        "demo.gender",
        {
            "m": {
                "id": "canonical.gender.male",
                "label": {"en": "Male"},
            },
        },
    )

    expr = (
        "code.map_or_default('demo.gender', type_string(source.raw_gender), "
        "'canonical.gender.unknown')"
    )
    value = rt.evaluate_expression(expr, {"raw_gender": "m"}, {})
    print(expr)
    pprint.pp(value)


if __name__ == "__main__":
    main()
