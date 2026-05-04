#!/usr/bin/env python3
"""
Compile-time vs input errors: ``MappingCompileError`` vs ``TypeError``.

Prerequisite: ``maturin develop`` in ``crates/cel-mapper-python``, then::

    python examples/error_handling.py
"""

from __future__ import annotations

from cel_mapper import MappingCompileError, MappingRuntime


def demo_compile_error() -> None:
    rt = MappingRuntime()
    bad_yaml = """
version: "0.1"
name: broken
records:
  r:
    fields:
      x: '1 +'   # invalid CEL
"""
    try:
        rt.compile_mapping(bad_yaml)
    except MappingCompileError as e:
        print("Caught MappingCompileError (compile-time):")
        print(e)
    else:
        raise SystemExit("expected MappingCompileError")


def demo_bad_source_type() -> None:
    rt = MappingRuntime()
    yaml = """
version: "0.1"
name: ok
records:
  r:
    fields:
      x: '"fine"'
"""
    try:
        # `None` is not a valid source payload
        rt.evaluate(yaml, None, {})
    except TypeError as e:
        print("\nCaught TypeError (bad Python input):")
        print(e)
    else:
        raise SystemExit("expected TypeError")


def main() -> None:
    demo_compile_error()
    demo_bad_source_type()


if __name__ == "__main__":
    main()
