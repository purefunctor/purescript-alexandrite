import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.direct, 42);
strictEqual(Main.firstClass(43), 43);
strictEqual(Main.indirect, 43);
