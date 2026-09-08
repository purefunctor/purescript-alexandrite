import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const tailAccumulator = Main.tailAccumulator(100_000);
strictEqual(tailAccumulator(0), 100_000);
strictEqual(tailAccumulator(10), 100_010);

strictEqual(Main.rotateArguments(100_001)(1)(2), 2);
