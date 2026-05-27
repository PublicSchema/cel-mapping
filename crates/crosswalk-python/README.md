# crosswalk (Python)

Native extension **`crosswalk`** (PyO3), built with **[Maturin](https://www.maturin.rs/)**.

Parent overview: **[`../../README.md`](../../README.md)**.

## Setup

```bash
cd crates/crosswalk-python
python3.13 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop --release
pytest -q
```

Supported Python versions are 3.10 through 3.13. The current PyO3 dependency
line does not support Python 3.14.

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

- **`pyproject.toml`** — `pip install .` / wheels via `maturin build`
- **`python/crosswalk/py.typed`** and **`__init__.pyi`** — typing hints

## API notes

- The JSON you pass as **`source`** becomes the CEL binding `source` (top-level object), not wrapped under a `"source"` key.
- **`preview_expression`** is intended for **editors**: it does not throw on syntax errors; inspect **`issues`** and **`notes`**.
