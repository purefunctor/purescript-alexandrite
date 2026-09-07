import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.namedBind("Unit")(), 42);
strictEqual(Main.tailBind(100_000)(), 0);
