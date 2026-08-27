import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.unsafelyCoerced, 42);
strictEqual(Main.safelyCoerced, 42);
strictEqual(Main.lookalikeUnsafeCoerce, 42);
strictEqual(Main.lookalikeSafeCoerce, 42);
