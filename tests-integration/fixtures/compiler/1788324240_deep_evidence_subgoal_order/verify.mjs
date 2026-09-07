import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { readTrace } from "./output/Main/foreign.js";

const Main = await import("./output/Main/index.js");

strictEqual(Main.result, 0);
deepStrictEqual(readTrace(), ["left", "right"]);
