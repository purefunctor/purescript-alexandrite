import { deepStrictEqual, strictEqual, throws } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

strictEqual(Main.first(Main.Empty), Main.Empty);
deepStrictEqual(Main.first(Main.One(1)), Main.One(1));
deepStrictEqual(Main.first(Main.Pair(2)(3)), Main.One(2));
deepStrictEqual(Main.pair(Main.Pair(4)(5)), Main.Pair(4)(5));
deepStrictEqual(Main.pair(Main.One(6)), Main.One(6));
strictEqual(Main.unwrap(7), 7);
deepStrictEqual(Main.nested(Main.Outer(Main.One(8))), Main.One(8));
strictEqual(Main.nested(Main.Outer(Main.Empty)), Main.Empty);
strictEqual(Main.ordinaryBind(9), 9);

const partialBind = Main.partialBind({});
strictEqual(partialBind(Main.One(10)), 10);
throws(() => partialBind(Main.Empty), { message: "Pattern match failure" });

const inapplicableOuterPattern = new Proxy(
  { tag: "One" },
  {
    get(target, property) {
      if (property === "_1") {
        throw new Error("extracted a constructor field before matching its tag");
      }
      return Reflect.get(target, property);
    },
  },
);
strictEqual(Main.nested(inapplicableOuterPattern), Main.Empty);
