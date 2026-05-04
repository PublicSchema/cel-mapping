/**
 * TypeScript entry for `cel-mapper-wasm` (web target).
 *
 * **Idiomatic surface:** {@link CelMapper} — `await CelMapper.create()`, camelCase helpers, objects in / parsed JSON out.
 *
 * **Low-level surface:** {@link init}, {@link CelMapperWasm}, {@link WasmMappingRuntime} — snake_case methods matching Rust
 * (`evaluate_json`, `evaluate_expression_json`, …) and JSON **strings** for wasm interop.
 *
 * Build: `npm run build:wasm` then `npm run build:ts` (see `README.md` in this package).
 */

import wasmInit, * as CelMapperWasm from "../wasm-pkg/cel_mapper_wasm.js";

export type { WasmMappingRuntime, InitInput } from "../wasm-pkg/cel_mapper_wasm.js";

/** Initialize WebAssembly (required once per page before using the runtime). */
export async function init(
  moduleOrPath?: CelMapperWasm.InitInput | Promise<CelMapperWasm.InitInput>
): Promise<void> {
  await wasmInit(moduleOrPath);
}

export { CelMapperWasm };

// --- JSON shapes returned by `WasmMappingRuntime` string methods (matches `serde_json` / core types) ---

/** Result of `evaluate_expression_json` after `JSON.parse`. */
export type EvaluateExpressionJson =
  | { ok: true; value: unknown }
  | { ok: false; error: string };

/** Phases for a single-expression diagnostic (`ExpressionIssue.phase`). */
export type ExpressionPhaseJson = "limits" | "syntax" | "evaluation";

/** One structured issue from `preview_expression_json`. */
export interface ExpressionIssueJson {
  phase: ExpressionPhaseJson;
  severity: "error" | "warning";
  /** Same string form as Rust `ErrorCode` in JSON (`SCREAMING_SNAKE_CASE`). */
  code: string;
  message: string;
  line?: number;
  column?: number;
  expression: string;
  source_path?: string;
}

/** Result of `preview_expression_json` after `JSON.parse` (Rust `ExpressionPreviewResult`). */
export interface ExpressionPreviewJson {
  author_expression: string;
  rewritten_expression?: string;
  value?: unknown | null;
  issues: ExpressionIssueJson[];
  notes: string[];
}

/**
 * Parse the JSON string returned by `WasmMappingRuntime.evaluate_expression_json`.
 * Throws if `json` is not valid JSON.
 */
export function parseEvaluateExpressionJson(json: string): EvaluateExpressionJson {
  return JSON.parse(json) as EvaluateExpressionJson;
}

/**
 * Parse the JSON string returned by `WasmMappingRuntime.preview_expression_json`.
 * Throws if `json` is not valid JSON.
 */
export function parsePreviewExpressionJson(json: string): ExpressionPreviewJson {
  return JSON.parse(json) as ExpressionPreviewJson;
}

/** Full mapping evaluation JSON (`evaluate_json` / {@link CelMapper.evaluate}). */
export interface MappingEvaluationResult {
  records: Record<string, unknown[]>;
  warnings: unknown[];
  errors: unknown[];
}

export type PublicSchemaDirection = "to_target" | "from_target";
export type PublicSchemaPrivacy = "production" | "authoring" | "debug";

/**
 * Spec-conformant status values for a single rule log entry.
 * Matches the statuses emitted by `evaluate_publicschema_mapping` in Rust core.
 */
export type PublicSchemaStatus =
  | "applied"
  | "defaulted"
  | "omitted"
  | "missing"
  | "skipped"
  | "formula_error"
  | "write_error"
  | "validation_error";

export interface PublicSchemaCompileMeta {
  mapping_id?: string | null;
  version: string;
  source?: string | null;
  target?: string | null;
  deterministic_hash: string;
  /** "canonical" or "advisory-unimplemented" per spec §11.1. */
  hash_status: string;
  binding_mode: string;
  property_mapping_count: number;
  expression_count: number;
  warnings: unknown[];
}

/** One entry in `PublicSchemaTransformResult.log`. */
export interface PublicSchemaRuleLogEntry {
  index: number;
  rule_id?: string | null;
  source_path: string;
  target_path: string;
  status: PublicSchemaStatus | string;
  expression?: string | null;
  resolved_input?: unknown;
  resolved_output?: unknown;
  quality?: string | null;
  issues?: unknown[];
}

export interface PublicSchemaTransformResult {
  ok: boolean;
  output: unknown;
  log: PublicSchemaRuleLogEntry[];
  warnings: unknown[];
  errors: unknown[];
}

/**
 * A PublicSchema property-mapping rule as accepted by
 * `previewPublicSchemaRuleExpression`. Matches the `property_mappings[]` item
 * shape in the mapping document (`source`, `target`, and optional `formula`).
 */
export interface PublicSchemaRule {
  source: string;
  target: string;
  formula?: unknown;
  id?: string;
  required?: boolean;
  [key: string]: unknown;
}

/** Shape of `getPublicSchemaHelperMetadata()` after `JSON.parse`. */
export interface PublicSchemaHelperMetadata {
  version: string;
  helpers: Array<{ name: string; [key: string]: unknown }>;
}

type PublicSchemaWasmRuntime = CelMapperWasm.WasmMappingRuntime & {
  compile_publicschema_mapping_meta(mapping: string): string;
  evaluate_publicschema_json(
    mapping: string,
    sourceJson: string,
    ctxJson: string,
    direction: string,
    errors_mode: string,
    privacy: string
  ): string;
  preview_publicschema_rule_expression_json(
    ruleJson: string,
    sourceJson: string,
    optionsJson: string
  ): string;
  get_publicschema_helper_metadata(): string;
};

function throwIfConfigError(raw: string, label: string): void {
  const j = JSON.parse(raw) as { error?: string };
  if (typeof j.error === "string") {
    throw new Error(`${label}: ${j.error}`);
  }
}

function parseCompileMeta(raw: string): { name: string; version: string } {
  const j = JSON.parse(raw) as { name?: string; version?: string; error?: string };
  if (typeof j.error === "string") {
    throw new Error(j.error);
  }
  if (typeof j.name !== "string" || typeof j.version !== "string") {
    throw new Error("compile_mapping_meta: unexpected response shape");
  }
  return { name: j.name, version: j.version };
}

function parsePublicSchemaCompileMeta(raw: string): PublicSchemaCompileMeta {
  const j = JSON.parse(raw) as PublicSchemaCompileMeta & { error?: string };
  if (typeof j.error === "string") {
    throw new Error(j.error);
  }
  if (typeof j.deterministic_hash !== "string" || typeof j.version !== "string") {
    throw new Error("compile_publicschema_mapping_meta: unexpected response shape");
  }
  return j;
}

/**
 * Ergonomic wrapper around {@link WasmMappingRuntime}: camelCase methods, `unknown` / objects for payloads,
 * parsed results (no manual `JSON.parse` for common flows). Throws on invalid limits / runtime options / compile meta.
 *
 * Prefer **`await CelMapper.create()`** in apps; use {@link CelMapper.wasm} when you need snake_case JSON strings.
 */
export class CelMapper {
  readonly #rt: CelMapperWasm.WasmMappingRuntime;

  private constructor(rt: CelMapperWasm.WasmMappingRuntime) {
    this.#rt = rt;
  }

  /**
   * Initializes WASM (same as {@link init}) and constructs a runtime. Pass `initInput` if you load the `.wasm`
   * from a non-default URL (see wasm-bindgen web docs).
   */
  static async create(
    initInput?: CelMapperWasm.InitInput | Promise<CelMapperWasm.InitInput>
  ): Promise<CelMapper> {
    await wasmInit(initInput);
    return new CelMapper(new CelMapperWasm.WasmMappingRuntime());
  }

  /** @throws if JSON does not match `SecurityLimits` */
  setLimits(limits: unknown): void {
    throwIfConfigError(this.#rt.set_limits_json(JSON.stringify(limits)), "setLimits");
  }

  /** @throws if JSON does not match host `RuntimeOptions` */
  setRuntimeOptions(options: unknown): void {
    throwIfConfigError(this.#rt.set_runtime_options_json(JSON.stringify(options)), "setRuntimeOptions");
  }

  /** @throws on invalid mapping YAML */
  compileMappingMeta(mappingYaml: string): { name: string; version: string } {
    return parseCompileMeta(this.#rt.compile_mapping_meta(mappingYaml));
  }

  /** @throws on invalid PublicSchema v0.2 mapping YAML/JSON */
  compilePublicSchemaMappingMeta(mapping: string): PublicSchemaCompileMeta {
    return parsePublicSchemaCompileMeta(
      (this.#rt as PublicSchemaWasmRuntime).compile_publicschema_mapping_meta(mapping)
    );
  }

  /**
   * Evaluate mapping YAML. `source` becomes the CEL binding `source` (top-level JSON object).
   * Recompiles the mapping every call — cache YAML client-side if it is fixed.
   */
  evaluate(
    mappingYaml: string,
    source: unknown,
    context: unknown = {}
  ): MappingEvaluationResult {
    const raw = this.#rt.evaluate_json(
      mappingYaml,
      JSON.stringify(source),
      JSON.stringify(context ?? {})
    );
    return JSON.parse(raw) as MappingEvaluationResult;
  }

  /** Evaluate PublicSchema v0.2 `property_mappings[]` with Rust-core semantics. */
  evaluatePublicSchemaMapping(
    mapping: string,
    source: unknown,
    context: unknown = {},
    options: {
      direction?: PublicSchemaDirection;
      errors_mode?: "strict" | "collect" | "lenient" | "";
      privacy?: PublicSchemaPrivacy;
    } = {}
  ): PublicSchemaTransformResult {
    const raw = (this.#rt as PublicSchemaWasmRuntime).evaluate_publicschema_json(
      mapping,
      JSON.stringify(source),
      JSON.stringify(context ?? {}),
      options.direction ?? "to_target",
      options.errors_mode ?? "",
      options.privacy ?? "production"
    );
    return JSON.parse(raw) as PublicSchemaTransformResult;
  }

  /**
   * Editor-oriented preview for a single PublicSchema property-mapping rule.
   * `rule` is a property-mapping object (`source`, `target`, optional `formula`).
   * `source` is the sample record. `options` may include `direction` and `context`.
   * Returns the same shape as {@link previewExpression}.
   */
  previewPublicSchemaRuleExpression(
    rule: PublicSchemaRule | Record<string, unknown>,
    source: unknown,
    options: {
      direction?: PublicSchemaDirection;
      context?: unknown;
    } = {}
  ): ExpressionPreviewJson {
    const optionsJson = JSON.stringify({
      direction: options.direction ?? "to_target",
      context: options.context ?? {},
    });
    const raw = (this.#rt as PublicSchemaWasmRuntime).preview_publicschema_rule_expression_json(
      JSON.stringify(rule),
      JSON.stringify(source),
      optionsJson
    );
    const parsed = JSON.parse(raw) as ExpressionPreviewJson & { error?: string };
    if (typeof parsed.error === "string") {
      throw new Error(`previewPublicSchemaRuleExpression: ${parsed.error}`);
    }
    return parsed;
  }

  /**
   * Returns helper-function registry metadata listing all available stdlib helpers.
   * Shape: `{ version: string; helpers: Array<{ name: string }> }`.
   */
  getPublicSchemaHelperMetadata(): PublicSchemaHelperMetadata {
    const raw = (this.#rt as PublicSchemaWasmRuntime).get_publicschema_helper_metadata();
    return JSON.parse(raw) as PublicSchemaHelperMetadata;
  }

  /** Single CEL expression (stdlib + `source` / `ctx`); see {@link EvaluateExpressionJson}. */
  evaluateExpression(
    expr: string,
    source: unknown,
    context: unknown = {}
  ): EvaluateExpressionJson {
    return parseEvaluateExpressionJson(
      this.#rt.evaluate_expression_json(expr, JSON.stringify(source), JSON.stringify(context ?? {}))
    );
  }

  /** Editor-oriented preview; see {@link ExpressionPreviewJson}. */
  previewExpression(
    expr: string,
    source: unknown,
    context: unknown = {}
  ): ExpressionPreviewJson {
    return parsePreviewExpressionJson(
      this.#rt.preview_expression_json(expr, JSON.stringify(source), JSON.stringify(context ?? {}))
    );
  }

  /** Underlying wasm-bindgen type (snake_case, JSON strings). */
  get wasm(): CelMapperWasm.WasmMappingRuntime {
    return this.#rt;
  }
}
