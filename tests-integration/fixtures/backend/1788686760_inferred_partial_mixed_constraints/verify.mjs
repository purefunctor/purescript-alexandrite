import { strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const selectFirst = { select: first => second => first };
const selectSecond = { select: first => second => second };
strictEqual(Main.mixed({})(selectFirst)(Main.Just(47))(48), 47);
strictEqual(Main.mixed({})(selectSecond)(Main.Just(47))(48), 48);
strictEqual(Main.discharged(Main.Just(49))(50), 50);
throws(() => Main.mixed({})(selectSecond)(Main.Nothing)(53), {
  message: "Pattern match failure",
});
throws(() => Main.discharged(Main.Nothing)(54), {
  message: "Pattern match failure",
});
