import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readCounts, resetCounts } from "./output/Main/foreign.js";

resetCounts();
const effect = Main.effectTail(100_000);
deepStrictEqual(readCounts(), { constructions: 1, runs: 0 });
strictEqual(effect(), 0);
deepStrictEqual(readCounts(), { constructions: 100_000, runs: 100_000 });
strictEqual(effect(), 0);
deepStrictEqual(readCounts(), { constructions: 199_999, runs: 200_000 });

resetCounts();
strictEqual(Main.effectMutualEven(100_000)(), true);
deepStrictEqual(readCounts(), { constructions: 100_000, runs: 100_000 });

resetCounts();
strictEqual(Main.effectMixedLong(100_000)(42)(), 0);
deepStrictEqual(readCounts(), { constructions: 100_000, runs: 100_000 });
