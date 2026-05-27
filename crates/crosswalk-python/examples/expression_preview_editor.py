#!/usr/bin/env python3
"""
Editor-style preview: ``author_expression``, ``rewritten_expression``, ``notes``, ``issues``, ``value``.

``notes`` explains that syntax ``line``/``column`` refer to ``rewritten_expression``; ``issues[].message``
includes CEL’s multi-line parse diagnostic (snippet + caret) when available.

Prerequisite: ``maturin develop`` in ``crates/crosswalk-python``, then::

    python examples/expression_preview_editor.py
"""

from __future__ import annotations

import json
import pprint

from crosswalk import MappingRuntime


def show(label: str, expr: str, source: dict) -> None:
    rt = MappingRuntime()
    out = rt.preview_expression(expr, source, {})
    print(f"\n=== {label} ===\nexpr: {expr!r}")
    pprint.pp(out)


def main() -> None:
    show("syntax error", "1 +", {})
    show("ok", "type_string(source.x)", {"x": 42})
    # Serializes cleanly for LSP / web worker JSON-RPC payloads:
    ok = MappingRuntime().preview_expression("type_string(1)", {}, {})
    print("\nJSON:", json.dumps(ok))


if __name__ == "__main__":
    main()
