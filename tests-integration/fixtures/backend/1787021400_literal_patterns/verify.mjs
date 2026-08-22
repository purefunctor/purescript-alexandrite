import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const actual = {
  escapedDoubleQuote: Main.escapedDoubleQuote,
  matchesEscapedDoubleQuote: Main.matchesEscapedDoubleQuote(Main.escapedDoubleQuote),
  rejectsOtherCharacter: Main.matchesEscapedDoubleQuote("x"),
};
const expected = {
  escapedDoubleQuote: '"',
  matchesEscapedDoubleQuote: true,
  rejectsOtherCharacter: false,
};

deepStrictEqual(actual, expected, "unexpected escaped double-quote character behavior");
