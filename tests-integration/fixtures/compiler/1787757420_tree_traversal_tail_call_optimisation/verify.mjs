import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

let tree = Main.Leaf(1);

for (let index = 0; index < 100_000; index += 1) {
  tree = Main.Branch(tree)(Main.Leaf(1));
}

strictEqual(Main.sumTree(tree), 100_001);
