import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.evaluateIf(false), 0);
strictEqual(Main.evaluateCase(false), 0);
strictEqual(Main.evaluateGuard(false), 0);

const expected = { message: "deep evidence was evaluated" };
throws(() => Main.evaluateIf(true), expected);
throws(() => Main.evaluateCase(true), expected);
throws(() => Main.evaluateGuard(true), expected);
