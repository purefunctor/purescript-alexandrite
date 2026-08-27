import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.mutualEven(100_000), true);
strictEqual(Main.mutualOdd(100_001), true);
