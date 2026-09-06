import { deepStrictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { Just, Nothing } from "./output/Library/index.js";

for (const forwarded of [Main.forwarded({}), Main.signed({}), Main.discharged]) {
  deepStrictEqual(forwarded(Just(41))(Just(42)), { first: 41, second: 42 });
  throws(() => forwarded(Nothing)(Just(42)), { message: "Pattern match failure" });
  throws(() => forwarded(Just(41))(Nothing), { message: "Pattern match failure" });
}
