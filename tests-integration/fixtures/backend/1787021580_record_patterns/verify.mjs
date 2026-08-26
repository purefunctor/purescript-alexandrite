import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.select({ first: 1, nested: { second: "selected" } }), "selected");
