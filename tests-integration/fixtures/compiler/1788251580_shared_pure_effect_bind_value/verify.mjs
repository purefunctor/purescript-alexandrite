import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

deepStrictEqual(Main.sharedBind("Unit")(), { left: 42, right: 42 });
