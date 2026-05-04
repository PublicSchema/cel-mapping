"""Smoke tests for the `cel_mapper` extension (run after `maturin develop` in this directory)."""

import pytest

pytest.importorskip("cel_mapper")

from cel_mapper import CompiledMapping, MappingCompileError, MappingRuntime  # noqa: E402


def test_compile_once_evaluate_many_dict_io():
    rt = MappingRuntime()
    yaml = """
version: "0.1"
name: t
records:
  r:
    fields:
      x: '"ok"'
"""
    compiled = rt.compile_mapping(yaml)
    assert isinstance(compiled, CompiledMapping)
    assert compiled.name == "t"
    assert compiled.version == "0.1"
    out1 = rt.evaluate_compiled(compiled, {}, {})
    out2 = rt.evaluate_compiled(compiled, {}, {})
    assert isinstance(out1, dict)
    assert out1 == out2
    assert out1["errors"] == []


def test_evaluate_string_json_source():
    rt = MappingRuntime()
    yaml = """
version: "0.1"
name: t
records:
  r:
    fields:
      x: '"ok"'
"""
    out = rt.evaluate(yaml, "{}", "{}")
    assert out["errors"] == []


def test_runtime_options_dict_constructor():
    rt = MappingRuntime({"default_errors_mode": "collect"})
    yaml = """
version: "0.1"
name: t
records:
  r:
    fields:
      a: 'type_int("x")'
"""
    compiled = rt.compile_mapping(yaml)
    out = rt.evaluate_compiled(compiled, {}, {})
    assert out["records"]["r"]


def test_runtime_options_json_string_constructor_backward_compat():
    rt = MappingRuntime('{"default_errors_mode": "collect"}')
    yaml = """
version: "0.1"
name: t
records:
  r:
    fields:
      a: 'type_int("x")'
"""
    compiled = rt.compile_mapping(yaml)
    out = rt.evaluate_compiled(compiled, {}, {})
    assert out["records"]["r"]


def test_set_limits_dict():
    rt = MappingRuntime()
    rt.set_limits(
        {
            "max_expression_bytes": 50_000,
            "max_output_json_bytes": 16 * 1024 * 1024,
            "max_list_len": 100_000,
            "max_string_bytes": 1024 * 1024,
            "max_eval_steps": 1_000_000,
        }
    )


def test_preview_expression_syntax_issue():
    rt = MappingRuntime()
    out = rt.preview_expression("1 +", {}, {})
    assert out["author_expression"] == "1 +"
    assert out["rewritten_expression"] == "1 +"
    assert out["notes"]
    assert out["value"] is None
    assert len(out["issues"]) == 1
    issue = out["issues"][0]
    assert issue["phase"] == "syntax"
    assert issue["line"] == 1
    assert issue["column"] is not None
    assert "ERROR:" in issue["message"] or "|" in issue["message"]


def test_preview_expression_ok():
    rt = MappingRuntime()
    out = rt.preview_expression("type_string(1)", {}, {})
    assert out["author_expression"] == "type_string(1)"
    assert out["rewritten_expression"] == "type_string(1)"
    assert out["notes"] == []
    assert out["issues"] == []
    assert out["value"] == "1"


def test_evaluate_expression_without_mapping_yaml():
    rt = MappingRuntime()
    rt.register_code_system(
        "demo.gender",
        {
            "m": {"id": "canon.m", "label": {"en": "Male"}},
        },
    )
    v = rt.evaluate_expression(
        "code.map_or_default('demo.gender', type_string(source.raw), 'canon.unknown')",
        {"raw": "m"},
        {},
    )
    assert v == "canon.m"


def test_mapping_compile_error():
    rt = MappingRuntime()
    yaml = """
version: "0.1"
name: t
records:
  r:
    fields:
      x: '1 +'
"""
    with pytest.raises(MappingCompileError):
        rt.compile_mapping(yaml)
