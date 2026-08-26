import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.unwrap(42), 42);
strictEqual(Main.select({ first: 1, second: "selected" }), "selected");

const unwrapOne = Main.unwrapOne({});
strictEqual(unwrapOne(Main.One(43)), 43);
throws(() => unwrapOne(Main.Empty), { message: "Pattern match failure" });
