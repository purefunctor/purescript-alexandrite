import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.functionIdentity, 42);
strictEqual(Main.lookalikeIdentity, 42);
