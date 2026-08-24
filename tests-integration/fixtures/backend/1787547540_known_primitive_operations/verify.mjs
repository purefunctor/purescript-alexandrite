import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.booleanNot(true), false);
strictEqual(Main.integerAdd(2147483647)(1), -2147483648);
strictEqual(Main.integerSubtract(-2147483647)(2), 2147483647);
strictEqual(Main.integerMultiply(65536)(65536), 0);
strictEqual(Main.integerNegate(-2147483648), -2147483648);
strictEqual(Main.partiallyAppliedAdd(41), 42);
strictEqual(Main.lookalikeAdd(20)(22), 42);

Main.readTrace(true);
strictEqual(Main.integerAddOrder(false), 42);
deepStrictEqual(Main.readTrace(true), ["left", "right"]);
