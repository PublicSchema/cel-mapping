"""
Parity tests for the PublicSchema v0.2 Python binding.

Run after ``maturin develop`` in ``crates/cel-mapper-python``.

Source records use plain object keys (no leading slash); the runtime resolves
JSON Pointer paths like ``/given_name`` against ``{"given_name": "..."}`` by
stripping the leading ``/`` and treating remaining segments as object keys.
"""

import pytest

pytest.importorskip("cel_mapper")

from cel_mapper import MappingCompileError, MappingRuntime  # noqa: E402

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

SIMPLE_MAPPING = """
version: "0.2"
id: test-mapping
source: src-system
target: tgt-system
property_mappings:
  - source: /given_name
    target: /first_name
  - source: /family_name
    target: /last_name
    required: true
"""

FORMULA_MAPPING = """
version: "0.2"
id: formula-mapping
property_mappings:
  - source: /raw_age
    target: /age_int
    formula:
      to_target:
        expression: "type_int(source)"
"""

DUPLICATE_TARGET_MAPPING = """
version: "0.2"
id: dup-target
property_mappings:
  - source: /a
    target: /x
  - source: /b
    target: /x
"""

RULE_ID_MAPPING = """
version: "0.2"
id: rule-id-test
property_mappings:
  - id: my-rule
    source: /val
    target: /out
"""


# ---------------------------------------------------------------------------
# 1. Identity copy (no formula)
# ---------------------------------------------------------------------------


def test_identity_copy():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(SIMPLE_MAPPING)
    # Source keys are plain (no leading slash); pointer /given_name resolves via key "given_name".
    out = rt.evaluate_publicschema_compiled(compiled, {"given_name": "Alice", "family_name": "Smith"})
    assert out["ok"] is True
    assert out["output"]["first_name"] == "Alice"
    assert out["output"]["last_name"] == "Smith"


# ---------------------------------------------------------------------------
# 2. Explicit ``source`` identity formula (same as no formula)
# ---------------------------------------------------------------------------


def test_explicit_source_formula():
    mapping = """
version: "0.2"
id: explicit-source
property_mappings:
  - source: /given_name
    target: /first_name
    formula:
      to_target:
        expression: source
"""
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(mapping)
    out = rt.evaluate_publicschema_compiled(compiled, {"given_name": "Bob"})
    assert out["ok"] is True
    assert out["errors"] == []
    assert out["output"]["first_name"] == "Bob"


# ---------------------------------------------------------------------------
# 3. Wrong-direction formula → ``formula_error``
# ---------------------------------------------------------------------------


def test_wrong_direction_formula_error():
    """Formula defined only for to_target; evaluating from_target must fail closed.

    In from_target direction the runtime reads from the target side (age_int)
    and writes to the source side (raw_age).  The formula only has a to_target
    branch, so this must produce formula_error, not identity-copy.
    """
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(FORMULA_MAPPING)
    # Provide the value on the target (age_int) side for a reverse transform.
    out = rt.evaluate_publicschema_compiled(compiled, {"age_int": 30}, direction="from_target")
    assert out["log"][0]["status"] == "formula_error"
    assert out["ok"] is False


# ---------------------------------------------------------------------------
# 4. Missing optional field → ``omitted``
# ---------------------------------------------------------------------------


def test_missing_optional_omitted():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(SIMPLE_MAPPING)
    # Provide family_name (required), omit given_name (optional).
    out = rt.evaluate_publicschema_compiled(compiled, {"family_name": "Jones"})
    assert out["ok"] is True
    given_name_log = next(e for e in out["log"] if e["source_path"] == "/given_name")
    assert given_name_log["status"] == "omitted"


# ---------------------------------------------------------------------------
# 5. Missing required field → ``missing`` and ok=False
# ---------------------------------------------------------------------------


def test_missing_required_not_ok():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(SIMPLE_MAPPING)
    # Omit the required family_name.
    out = rt.evaluate_publicschema_compiled(compiled, {"given_name": "Alice"})
    assert out["ok"] is False
    family_name_log = next(e for e in out["log"] if e["source_path"] == "/family_name")
    assert family_name_log["status"] == "missing"


# ---------------------------------------------------------------------------
# 6. Formula error fails closed (no identity fallback)
# ---------------------------------------------------------------------------


def test_formula_error_no_identity_fallback():
    """A runtime formula error must not silently fall back to the source value."""
    mapping = """
version: "0.2"
id: bad-formula
property_mappings:
  - source: /val
    target: /out
    formula:
      to_target:
        expression: "undefined_fn(source)"
"""
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(mapping)
    out = rt.evaluate_publicschema_compiled(compiled, {"val": "hello"})
    assert out["log"][0]["status"] == "formula_error"
    # No output written for the errored field.
    assert "out" not in out["output"]


# ---------------------------------------------------------------------------
# 7. Duplicate target: last-write-wins with warning
# ---------------------------------------------------------------------------


def test_duplicate_target_last_write_wins():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(DUPLICATE_TARGET_MAPPING)
    out = rt.evaluate_publicschema_compiled(compiled, {"a": "first", "b": "second"})
    # Last write wins (second rule overwrites first).
    assert out["output"]["x"] == "second"
    # At least one warning about the duplicate target.
    dup_warnings = [w for w in out["warnings"] if "last write wins" in w.get("message", "")]
    assert len(dup_warnings) >= 1


# ---------------------------------------------------------------------------
# 8. Hash status and label
# ---------------------------------------------------------------------------


def test_hash_status_canonical():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(SIMPLE_MAPPING)
    meta = compiled.meta()
    assert meta["hash_status"] == "canonical"
    assert meta["deterministic_hash"]  # non-empty hex string
    # Hash is deterministic: compiling the same YAML twice gives the same hash.
    compiled2 = rt.compile_publicschema_mapping(SIMPLE_MAPPING)
    assert compiled.deterministic_hash == compiled2.deterministic_hash


# ---------------------------------------------------------------------------
# 9. preview_publicschema_rule_expression: valid rule returns rewritten expression
# ---------------------------------------------------------------------------


def test_preview_publicschema_rule_valid():
    """The sample passed to preview is the resolved field value (what 'source' binds to)."""
    rt = MappingRuntime()
    rule = {
        "source": "/given_name",
        "target": "/first_name",
        "formula": {
            "to_target": {"expression": "type_string(source)"},
        },
    }
    # Pass the field value as sample_record ('"Alice"' is valid JSON string).
    result = rt.preview_publicschema_rule_expression(
        rule,
        '"Alice"',
        direction="to_target",
    )
    assert result["issues"] == []
    assert result["rewritten_expression"] is not None
    assert result["value"] == "Alice"


# ---------------------------------------------------------------------------
# 10. preview_publicschema_rule_expression: syntax-broken expression → issues
# ---------------------------------------------------------------------------


def test_preview_publicschema_rule_syntax_error():
    rt = MappingRuntime()
    rule = {
        "source": "/val",
        "target": "/out",
        "formula": {
            "to_target": {"expression": "1 +"},
        },
    }
    result = rt.preview_publicschema_rule_expression(
        rule,
        42,
        direction="to_target",
    )
    assert len(result["issues"]) >= 1
    assert result["issues"][0]["phase"] == "syntax"


# ---------------------------------------------------------------------------
# 11. preview_publicschema_rule_expression: unknown direction → TypeError
# ---------------------------------------------------------------------------


def test_preview_publicschema_rule_unknown_direction_raises():
    rt = MappingRuntime()
    rule = {"source": "/x", "target": "/y"}
    with pytest.raises(TypeError):
        rt.preview_publicschema_rule_expression(rule, 1, direction="sideways")


# ---------------------------------------------------------------------------
# 12. Log entries use spec-correct field names
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# 13. helper_registry_version returns a non-empty string
# ---------------------------------------------------------------------------


def test_helper_registry_version_non_empty():
    rt = MappingRuntime()
    version = rt.helper_registry_version()
    assert isinstance(version, str)
    assert version, "helper_registry_version() must return a non-empty string"


def test_log_entry_field_names():
    rt = MappingRuntime()
    compiled = rt.compile_publicschema_mapping(RULE_ID_MAPPING)
    out = rt.evaluate_publicschema_compiled(compiled, {"val": "hello"})
    entry = out["log"][0]
    # Spec-correct names must be present.
    assert "source_path" in entry
    assert "target_path" in entry
    # rule_id is populated from the ``id`` field.
    assert entry.get("rule_id") == "my-rule"
    # Old names must NOT appear.
    assert "source" not in entry
    assert "target" not in entry
    assert "formula" not in entry
