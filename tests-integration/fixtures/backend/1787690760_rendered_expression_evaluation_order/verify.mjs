import { deepStrictEqual, strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

deepStrictEqual(Main.reused, [4, 4]);
deepStrictEqual(readTrace(), ["observe:reused"]);

resetTrace();
deepStrictEqual(Main.ordered(true)(false), [1, 2, 3]);
deepStrictEqual(readTrace(), [
  "read:record",
  "observe:before",
  "collect:first:1",
  "fail:middle",
  "observe:branch-true",
  "collect:second:2",
  "observe:after",
  "collect:third:3",
]);

resetTrace();
throws(() => Main.ordered(false)(true), error => {
  strictEqual(error.message, "middle");
  return true;
});
deepStrictEqual(readTrace(), [
  "read:record",
  "observe:before",
  "collect:first:1",
  "fail:middle",
]);
