from typing import Any, Dict, Mapping, Optional, Union

_JSONish = Union[str, Mapping[str, Any]]

class MappingCompileError(Exception):
    """Raised when mapping YAML or embedded CEL fails to compile."""

class CompiledMapping:
    @property
    def name(self) -> str: ...
    @property
    def version(self) -> str: ...

class CompiledPublicSchemaMapping:
    @property
    def deterministic_hash(self) -> str: ...
    @property
    def version(self) -> str: ...
    def meta(self) -> Dict[str, Any]: ...

class MappingRuntime:
    def __init__(self, runtime_options: Optional[_JSONish] = None) -> None: ...
    def set_limits(self, limits: _JSONish) -> None: ...
    def set_runtime_options(self, options: _JSONish) -> None: ...
    def set_limits_json(self, json: str) -> None: ...
    def set_runtime_options_json(self, json: str) -> None: ...
    def register_code_system(self, name: str, spec: _JSONish) -> None: ...
    def compile_mapping(self, yaml: str) -> CompiledMapping: ...
    def compile_publicschema_mapping(self, mapping: str) -> CompiledPublicSchemaMapping: ...
    def helper_registry_version(self) -> str: ...
    def evaluate_compiled(
        self,
        compiled: CompiledMapping,
        source: _JSONish,
        context: Optional[_JSONish] = None,
    ) -> Dict[str, Any]: ...
    def evaluate_compiled_json(
        self, compiled: CompiledMapping, source_json: str, ctx_json: str
    ) -> str: ...
    def evaluate_publicschema_compiled(
        self,
        compiled: CompiledPublicSchemaMapping,
        source: _JSONish,
        context: Optional[_JSONish] = None,
        direction: Optional[str] = None,
        errors_mode: Optional[str] = None,
        privacy: Optional[str] = None,
    ) -> Dict[str, Any]: ...
    def evaluate_publicschema_compiled_json(
        self,
        compiled: CompiledPublicSchemaMapping,
        source_json: str,
        ctx_json: str,
        direction: Optional[str] = None,
        errors_mode: Optional[str] = None,
        privacy: Optional[str] = None,
    ) -> str: ...
    def evaluate(
        self,
        mapping_yaml: str,
        source: _JSONish,
        context: Optional[_JSONish] = None,
    ) -> Dict[str, Any]: ...
    def evaluate_json(
        self, mapping_yaml: str, source_json: str, ctx_json: str
    ) -> str: ...
    def evaluate_publicschema(
        self,
        mapping: str,
        source: _JSONish,
        context: Optional[_JSONish] = None,
        direction: Optional[str] = None,
        errors_mode: Optional[str] = None,
        privacy: Optional[str] = None,
    ) -> Dict[str, Any]: ...
    def evaluate_publicschema_json(
        self,
        mapping: str,
        source_json: str,
        ctx_json: str,
        direction: Optional[str] = None,
        errors_mode: Optional[str] = None,
        privacy: Optional[str] = None,
    ) -> str: ...
    def evaluate_expression(
        self,
        expr: str,
        source: _JSONish,
        context: Optional[_JSONish] = None,
    ) -> object: ...
    def preview_expression(
        self,
        expr: str,
        source: _JSONish,
        context: Optional[_JSONish] = None,
    ) -> Dict[str, Any]: ...
    def preview_publicschema_rule_expression(
        self,
        rule: _JSONish,
        source: _JSONish,
        *,
        direction: Optional[str] = None,
        context: Optional[_JSONish] = None,
    ) -> Dict[str, Any]: ...
