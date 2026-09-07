import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.booleanNot(true), false);
strictEqual(Main.integerAdd(2147483647)(1), -2147483648);
strictEqual(Main.integerSubtract(-2147483647)(2), 2147483647);
strictEqual(Main.integerMultiply(65536)(65536), 0);
strictEqual(Main.integerNegate(-2147483648), -2147483648);
strictEqual(Main.integerNegateLiteral, -20);
strictEqual(Main.inlineIntegerNegateLiteral, -20);
strictEqual(Main.numberNegate(20.5), -20.5);
strictEqual(Main.numberNegateLiteral, -20.5);
strictEqual(Main.numberNegateZero, 0);
for (const value of [0, -0, Infinity, -Infinity, NaN, Number.MIN_VALUE]) {
  strictEqual(Main.numberNegate(value), 0 - value);
}
strictEqual(Main.partiallyAppliedNegate(20), -20);
strictEqual(Main.partiallyAppliedAdd(41), 42);
strictEqual(Main.lookalikeAdd(20)(22), 42);
strictEqual(Main.lookalikeNegate(20), 20);
strictEqual(Main.genericNegate({
  Semiring0: () => ({ zero: 100 }),
  sub: left => right => left - right,
})(20), 80);

Main.readTrace(true);
strictEqual(Main.integerAddOrder(false), 42);
deepStrictEqual(Main.readTrace(true), ["left", "right"]);
