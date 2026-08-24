import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { readFile } from "node:fs/promises";
import * as Main from "./output/Main/index.js";

const source = await readFile(new URL("./output/Main/index.js", import.meta.url), "utf8");
if (/Data_Function|Control_Category|Unsafe_Coerce/.test(source)) {
  throw new Error("known function reduction left a canonical runtime reference");
}
for (const expected of ["Lookalike.apply", "Lookalike.categoryFn", "Lookalike.unsafeCoerce"]) {
  if (!source.includes(expected)) {
    throw new Error(`same-named non-canonical function was reduced: ${expected}`);
  }
}

strictEqual(Main.directApply, 42);
strictEqual(Main.flippedApply, 42);
strictEqual(Main.functionIdentity, 42);
strictEqual(Main.coerced, 42);
strictEqual(Main.lookalikeApply, 42);
strictEqual(Main.lookalikeIdentity, 42);
strictEqual(Main.lookalikeCoerce, 42);

Main.readTrace(true);
strictEqual(Main.directApplyOrder(false), 42);
deepStrictEqual(Main.readTrace(true), ["function", "argument"]);

strictEqual(Main.flippedApplyOrder(false), 42);
deepStrictEqual(Main.readTrace(true), ["argument", "function"]);
