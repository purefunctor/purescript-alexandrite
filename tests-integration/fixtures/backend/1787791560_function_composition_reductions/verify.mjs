import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.composed, 42);
strictEqual(Main.flippedComposed, 42);
strictEqual(Main.lookalikeCompose, 42);
strictEqual(Main.lookalikeComposeFlipped, 42);

Main.readTrace(true);
for (const [compose, expectedTrace] of [
  [Main.composeOrder, ["outer", "inner", "argument"]],
  [Main.partiallyComposedOrder, ["outer", "inner", "argument"]],
  [Main.flippedComposeOrder, ["inner", "outer", "argument"]],
  [Main.partiallyFlippedComposedOrder, ["inner", "outer", "argument"]],
]) {
  strictEqual(compose(false), 42);
  deepStrictEqual(Main.readTrace(true), expectedTrace);
}
