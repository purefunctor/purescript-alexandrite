import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.directRun, 42);
strictEqual(Main.directMadeRun, 42);
strictEqual(Main.directNestedRun, 42);
strictEqual(Main.directCapturedRun, 42);
strictEqual(Main.directCurriedResultRun, 42);
strictEqual(Main.partialRun(42), 42);
strictEqual(Main.indirectMake(1, 42), 42);
strictEqual(Main.lookalikeRun, 42);
