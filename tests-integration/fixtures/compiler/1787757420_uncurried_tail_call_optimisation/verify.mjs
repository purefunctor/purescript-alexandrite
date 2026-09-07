import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.uncurriedTail(100_000, 0), 100_000);
