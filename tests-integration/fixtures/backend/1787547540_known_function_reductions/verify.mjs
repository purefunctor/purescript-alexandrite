import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

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
