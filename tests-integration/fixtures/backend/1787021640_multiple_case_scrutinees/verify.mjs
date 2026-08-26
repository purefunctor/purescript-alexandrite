import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.choose(true)(true), 2);
strictEqual(Main.choose(true)(false), 1);
strictEqual(Main.choose(false)(true), 0);
strictEqual(Main.choose(false)(false), 0);
