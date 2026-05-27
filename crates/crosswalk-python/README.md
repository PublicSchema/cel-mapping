# crosswalk-python

`crosswalk-python` is the PyO3 extension crate that publishes the Python module
`crosswalk`. It wraps `crosswalk-core` for Python applications and tests.

Parent overview: **[`../../README.md`](../../README.md)**.

## When to use it

Use this package when Python code needs to compile or evaluate Crosswalk mapping
YAML, evaluate standalone CEL expressions, preview editor diagnostics, or
register code systems.

Rust code should depend on `crosswalk-core` directly. Browser and TypeScript
code should use `crosswalk-js`.

## Install

```bash
cd crates/crosswalk-python
python3.13 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop --release
pytest -q
```

Supported Python versions are 3.10 through 3.13. The current PyO3 dependency
line does not support Python 3.14.

For `uv` workflows:

```bash
cd crates/crosswalk-python
uv run --extra dev python -m pytest
```

## Quick start

```python
from crosswalk import MappingRuntime

yaml = """
version: "0.1"
name: demo
records:
  people:
    fields:
      name: "source.name"
"""

rt = MappingRuntime()
compiled = rt.compile_mapping(yaml)
out = rt.evaluate_compiled(compiled, {"name": "Ada"}, {})
assert out["records"]["people"][0]["name"] == "Ada"
```

## Examples

Runnable scripts (same directory):

```bash
for f in examples/*.py; do python "$f"; done
```

| Example | Shows |
|---------|--------|
| `compile_once_evaluate_many.py` | `compile_mapping` + `evaluate_compiled` |
| `one_shot_evaluate.py` | `evaluate` (dict in/out) |
| `error_handling.py` | `MappingCompileError` vs bad input |
| `register_code_system.py` | `register_code_system` + mapping |
| `evaluate_expression_only.py` | single CEL expression, no YAML |
| `expression_preview_editor.py` | `preview_expression` (structured issues + `author_expression` / `rewritten_expression`) |

## Packaging

- **`pyproject.toml`**: `pip install .` / wheels via `maturin build`
- **`python/crosswalk/py.typed`** and **`__init__.pyi`**: typing hints

## Public API surface

Runtime:

- `MappingRuntime`
- `CompiledMapping`
- `MappingCompileError`

Mapping APIs:

- `evaluate`
- `compile_mapping`
- `evaluate_compiled`
- `evaluate_json`

Standalone expression APIs:

- `evaluate_expression`
- `preview_expression`

Code-system and configuration APIs:

- `register_code_system`
- `set_limits_json`
- `set_runtime_options_json`

## Boundaries

This crate should stay a Python binding layer around `crosswalk-core`. Runtime
semantics belong in Rust crates below it. Python-specific work should focus on
argument conversion, exception shape, typing stubs, examples, and packaging.

## API notes

- The JSON you pass as **`source`** becomes the CEL binding `source` (top-level object), not wrapped under a `"source"` key.
- **`preview_expression`** is intended for **editors**: it does not throw on syntax errors; inspect **`issues`** and **`notes`**.

## Testing

```bash
cd crates/crosswalk-python
uv run --extra dev python -m pytest
uv run --with 'maturin>=1.5,<2' maturin build --release -m Cargo.toml
```
