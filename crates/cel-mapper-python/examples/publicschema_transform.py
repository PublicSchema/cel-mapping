#!/usr/bin/env python3
"""
PublicSchema v0.2 transform example.

Demonstrates:
- Compiling a PublicSchema mapping from YAML
- Evaluating with ``evaluate_publicschema_compiled`` and reading output/log
- Using ``preview_publicschema_rule_expression`` for a single rule
- Reading ``meta.deterministic_hash`` and ``meta.hash_status``

Prerequisite: ``maturin develop`` in ``crates/cel-mapper-python``, then::

    python examples/publicschema_transform.py
"""

from __future__ import annotations

import pprint

from cel_mapper import MappingRuntime

MAPPING_YAML = """
version: "0.2"
id: odk-to-profile
source: fieldbridge-odk
target: profile
property_mappings:
  - id: given-name
    source: /answers/respondent_first
    target: /given_name
    quality: exact
    formula:
      to_target:
        expression: source
      from_target:
        expression: source
  - id: age-int
    source: /answers/age
    target: /age
    formula:
      to_target:
        expression: "type_int(source)"
  - id: email
    source: /answers/email
    target: /email
    required: true
"""


def main() -> None:
    rt = MappingRuntime()

    # --- Compile once, reuse ---
    compiled = rt.compile_publicschema_mapping(MAPPING_YAML)

    # Deterministic hash and hash_status are part of the compile metadata.
    meta = compiled.meta()
    print("=== Compile metadata ===")
    print(f"  mapping_id   : {meta['mapping_id']}")
    print(f"  version      : {meta['version']}")
    print(f"  hash_status  : {meta['hash_status']}")
    print(f"  hash         : {meta['deterministic_hash']}")
    print(f"  rules        : {meta['property_mapping_count']}")
    print()

    # --- Forward transform ---
    # Source keys are plain object keys; the runtime resolves JSON Pointer paths
    # like ``/answers/respondent_first`` as nested object access: source["answers"]["respondent_first"].
    source = {
        "answers": {
            "respondent_first": "Alice",
            "age": "34",
            "email": "alice@example.com",
        }
    }
    out = rt.evaluate_publicschema_compiled(compiled, source)

    print("=== evaluate_publicschema_compiled output ===")
    pprint.pp(out["output"])
    print()

    print("=== Transform log ===")
    for entry in out["log"]:
        print(
            f"  [{entry['index']}] {entry['source_path']} -> {entry['target_path']}"
            f"  status={entry['status']}"
            + (f"  rule_id={entry['rule_id']}" if entry.get("rule_id") else "")
        )
    print()

    # --- Preview a single rule expression ---
    age_rule = {
        "source": "/answers/age",
        "target": "/age",
        "formula": {
            "to_target": {"expression": "type_int(source)"},
        },
    }
    # For preview, sample_record is the already-resolved field value that ``source`` binds to
    # in the CEL expression (not the whole source document).
    preview = rt.preview_publicschema_rule_expression(
        age_rule,
        '"34"',
        direction="to_target",
    )

    print("=== preview_publicschema_rule_expression (age-int rule) ===")
    print(f"  author_expression   : {preview['author_expression']}")
    print(f"  rewritten_expression: {preview['rewritten_expression']}")
    print(f"  value               : {preview['value']!r}")
    print(f"  issues              : {preview['issues']}")
    print()

    # --- Preview a rule with a syntax error ---
    broken_rule = {
        "source": "/answers/age",
        "target": "/age",
        "formula": {
            "to_target": {"expression": "type_int("},
        },
    }
    broken_preview = rt.preview_publicschema_rule_expression(
        broken_rule,
        '"34"',
        direction="to_target",
    )
    print("=== preview_publicschema_rule_expression (broken rule) ===")
    print(f"  issues: {broken_preview['issues']}")


if __name__ == "__main__":
    main()
