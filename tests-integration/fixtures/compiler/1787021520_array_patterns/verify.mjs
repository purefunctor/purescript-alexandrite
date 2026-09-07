import { strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.describe([]), 0);
strictEqual(Main.describe([42]), 42);
strictEqual(Main.describe([1, 2]), 1);
strictEqual(Main.describe([1, 2, 3]), 3);

const nonArray = new Proxy(
  {},
  {
    get(target, property) {
      if (property === "length" || property === "0") {
        throw new Error("extracted an array field before checking its shape");
      }
      return Reflect.get(target, property);
    },
  },
);
strictEqual(Main.describe(nonArray), 3);
