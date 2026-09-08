import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const unwrap = Main.local({});
strictEqual(unwrap(Main.Just(44)), 44);
throws(() => unwrap(Main.Nothing), { message: "Pattern match failure" });
