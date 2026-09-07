import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.singleArgument(100_001), 0);
strictEqual(Main.twoArguments(100_000)(42), 0);
