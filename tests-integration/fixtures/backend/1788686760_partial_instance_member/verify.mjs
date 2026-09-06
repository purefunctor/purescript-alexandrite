import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.discharged(Main.Just(46)), 46);
throws(() => Main.discharged(Main.Nothing), { message: "Pattern match failure" });
