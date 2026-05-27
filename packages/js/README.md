# crosswalk-js

TypeScript helpers and **`wasm-pack`** output for **`crosswalk-wasm`** (web target).

## Is the API idiomatic?

- **Low-level:** `WasmMappingRuntime` mirrors Rust: **snake_case** method names and **JSON strings** in/out. That is normal for raw `wasm-bindgen`, not especially idiomatic for application TypeScript.
- **High-level:** **`Crosswalk`** is the idiomatic surface: **`await Crosswalk.create()`**, **camelCase** methods, **`unknown` / objects** for `source` / `context`, parsed **`MappingEvaluationResult`** and preview types. Use it in apps; drop to **`.wasm`** when you need full control.

## Build

From **`packages/js`** (so `wasm-pack` `--out-dir` is correct):

```bash
npm ci
npm test
```

`npm test` builds the WASM package, builds TypeScript, and runs Node smoke tests
against the generated wrapper. The WASM build script prefers rustup-managed Rust
when it is installed so the `wasm32-unknown-unknown` target is visible.

## Example 1 — evaluate a mapping (high-level)

```typescript
import { Crosswalk } from "crosswalk-js";

const yaml = `
version: "0.1"
name: demo
records:
  out:
    fields:
      hello: "source.name"
`;

const mapper = await Crosswalk.create();
mapper.setRuntimeOptions({ default_errors_mode: "collect" });

const out = mapper.evaluate(yaml, { name: "Ada" }, { today: "2026-05-03" });
if (out.errors.length) {
  console.error(out.errors);
} else {
  console.log(out.records); // { out: [ { hello: "Ada" } ] } when evaluation succeeds
}
```

## Example 2 — mapping metadata before evaluate

```typescript
import { Crosswalk } from "crosswalk-js";

const mapper = await Crosswalk.create();
const meta = mapper.compileMappingMeta(mappingYaml);
console.log(meta.name, meta.version);
```

## Example 3 — editor: preview a raw CEL expression

```typescript
import { Crosswalk } from "crosswalk-js";

const mapper = await Crosswalk.create();
const preview = mapper.previewExpression("source.count + 1", { count: 2 }, {});

if (preview.issues.length) {
  console.warn(preview.issues, preview.notes);
} else {
  console.log(preview.value);
}
```

## Example 4 — probe one expression (no mapping YAML)

```typescript
import { Crosswalk } from "crosswalk-js";

const mapper = await Crosswalk.create();
const r = mapper.evaluateExpression("source.x * 2", { x: 21 }, {});
if (r.ok) console.log(r.value);
else console.error(r.error);
```

## Example 5 — PublicSchema value mappings

```typescript
import { Crosswalk } from "crosswalk-js";

const mapper = await Crosswalk.create();
const result = mapper.evaluatePublicSchemaMapping(mappingYaml, { sex: "U" }, {}, {
  errors_mode: "collect",
  privacy: "authoring",
});

if (result.log.some((entry) => entry.status === "value_unmapped")) {
  console.warn(result.errors);
}
```

`PublicSchemaStatus` includes `value_unmapped`. The runtime uses it when a
`value_mappings` crosswalk has no deterministic row, including ambiguous reverse
lookups where multiple `source_value`s share the same `target_value`.

## Example 6 — low-level wasm (snake_case + strings)

```typescript
import { init, CrosswalkWasm } from "crosswalk-js";

await init();
const rt = new CrosswalkWasm.WasmMappingRuntime();
const json = rt.evaluate_json(
  mappingYaml,
  JSON.stringify({ x: 1 }),
  JSON.stringify({ today: "2026-05-03" }),
);
const out = JSON.parse(json);
```

## Bundlers

Serve **`wasm-pkg/*.wasm`** from your static root (or pass an explicit URL/module into **`Crosswalk.create(fetch("…/crosswalk_wasm_bg.wasm"))`** per wasm-bindgen web init rules). Vite / Next / etc. usually need a rule to treat `.wasm` as a URL asset.
