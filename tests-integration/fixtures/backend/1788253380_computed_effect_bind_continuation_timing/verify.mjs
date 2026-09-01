import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readConstructions, resetConstructions } from "./output/Main/foreign.js";

resetConstructions();
const effect = Main.computedBind("Unit");
strictEqual(readConstructions(), 1);
strictEqual(effect(), 42);
strictEqual(readConstructions(), 1);
strictEqual(effect(), 42);
strictEqual(readConstructions(), 1);
