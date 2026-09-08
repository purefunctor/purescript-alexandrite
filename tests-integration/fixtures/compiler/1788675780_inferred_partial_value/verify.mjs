import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const partial = {};
strictEqual(Main.test(partial), 1);
strictEqual(Main.consumed(partial), 1);
strictEqual(Main.discharged, 1);
