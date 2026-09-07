import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.unwrapped, 42, "derived Newtype dictionary did not supply coercion evidence");
deepStrictEqual(Main.emptyRoundTrip, Main.Empty, "Generic to/from failed for the left branch");
deepStrictEqual(
  Main.singleRoundTrip,
  Main.Single(6),
  "Generic to/from failed for the middle branch",
);
deepStrictEqual(
  Main.pairRoundTrip,
  Main.Pair(7)(8),
  "Generic to/from failed for the right branch",
);
