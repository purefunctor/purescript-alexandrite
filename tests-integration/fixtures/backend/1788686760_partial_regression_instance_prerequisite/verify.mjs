import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

deepStrictEqual(Main.ordinaryPrerequisite, Main.Wrapped(20));
const selectFirst = { select: first => second => first };
deepStrictEqual(
  Main.selectWrapped(selectFirst).select(Main.Wrapped(51))(Main.Wrapped(52)),
  Main.Wrapped(51),
);
