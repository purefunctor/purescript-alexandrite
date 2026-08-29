import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { readFile } from "node:fs/promises";
import * as Main from "./output/Main/index.js";

const source = await readFile(new URL("./output/Main/index.js", import.meta.url), "utf8");
const expectedPrimitives = [
  "!value",
  "left + right | 0",
  "left - right | 0",
  "left * right | 0",
  "-value | 0",
  "return -value;",
];
for (const expected of expectedPrimitives) {
  if (!source.includes(expected)) {
    throw new Error(`missing known primitive output: ${expected}`);
  }
}
if (!source.includes("Lookalike.add(Lookalike.semiringInt)")) {
  throw new Error("same-named non-canonical member was reduced");
}

strictEqual(Main.booleanNot(true), false);
strictEqual(Main.integerAdd(2147483647)(1), -2147483648);
strictEqual(Main.integerSubtract(-2147483647)(2), 2147483647);
strictEqual(Main.integerMultiply(65536)(65536), 0);
strictEqual(Main.integerNegate(-2147483648), -2147483648);
strictEqual(Main.integerNegateLiteral, -20);
strictEqual(Main.inlineIntegerNegateLiteral, -20);
strictEqual(Main.numberNegate(20.5), -20.5);
strictEqual(Main.numberNegateLiteral, -20.5);
strictEqual(Main.partiallyAppliedAdd(41), 42);
strictEqual(Main.lookalikeAdd(20)(22), 42);

Main.readTrace(true);
strictEqual(Main.integerAddOrder(false), 42);
deepStrictEqual(Main.readTrace(true), ["left", "right"]);
