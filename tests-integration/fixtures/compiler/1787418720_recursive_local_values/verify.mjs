import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { resetTrace } from "./output/Main/foreign.js";

resetTrace();
deepStrictEqual(Main.applicationRecursive(false), {
  result: 2,
  trace: ["first", "second"],
});
strictEqual(Main.recordRecursive, true);
strictEqual(Main.caseRecursive(true), 30);
strictEqual(Main.caseRecursive(false), 31);
strictEqual(Main.letRecursive(true), 40);

let strictCycleError;
try {
  Main.strictCycle(false);
} catch (error) {
  strictCycleError = error;
}
if (
  !(strictCycleError instanceof ReferenceError) ||
  strictCycleError.message !== "value was needed before it finished initializing"
) {
  throw new Error(`unexpected strict-cycle result: ${strictCycleError}`);
}
