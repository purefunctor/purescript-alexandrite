import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

for (const unwrap of [Main.inferred({}), Main.signed({}), Main.discharged]) {
  strictEqual(unwrap(Main.Just(42)), 42);
  throws(() => unwrap(Main.Nothing), { message: "Pattern match failure" });
}
