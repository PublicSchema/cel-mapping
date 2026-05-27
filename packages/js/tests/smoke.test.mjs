import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  CelMapper,
  parseEvaluateExpressionJson,
  parsePreviewExpressionJson,
} from "../dist/index.js";

test("parses low-level JSON responses", () => {
  assert.deepEqual(parseEvaluateExpressionJson('{"ok":true,"value":"Ada"}'), {
    ok: true,
    value: "Ada",
  });

  assert.deepEqual(
    parsePreviewExpressionJson(
      '{"author_expression":"source.name","value":"Ada","issues":[],"notes":[]}'
    ),
    {
      author_expression: "source.name",
      value: "Ada",
      issues: [],
      notes: [],
    }
  );
});

test("initializes generated WASM and exercises the public wrapper", async () => {
  const wasmBytes = await readFile(
    new URL("../wasm-pkg/cel_mapper_wasm_bg.wasm", import.meta.url)
  );
  const mapper = await CelMapper.create({ module_or_path: wasmBytes });

  assert.deepEqual(mapper.evaluateExpression("source.name", { name: "Ada" }), {
    ok: true,
    value: "Ada",
  });

  const metadata = mapper.getPublicSchemaHelperMetadata();
  const helperNames = metadata.helpers.map((helper) => helper.name);

  assert.equal(metadata.version, "builtin");
  assert.ok(helperNames.includes("date_is_valid"));
  assert.ok(helperNames.includes("text_regex_extract"));
});
