import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.booleanGuard(true), 1);
strictEqual(Main.booleanGuard(false), 0);
strictEqual(Main.patternGuard(Main.One(42)), 42);
strictEqual(Main.patternGuard(Main.Empty), 0);
strictEqual(Main.caseBooleanGuard(true), 2);
strictEqual(Main.casePatternGuard(Main.One(43)), 43);
strictEqual(Main.casePatternGuard(Main.Empty), 0);
strictEqual(Main.nestedCaseGuard(true), 2);
strictEqual(Main.nestedCaseGuard(false), 3);
