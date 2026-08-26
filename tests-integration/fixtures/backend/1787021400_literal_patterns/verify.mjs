import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const actual = {
  integer: [Main.integer(0), Main.integer(1)],
  number: [Main.number(1.5), Main.number(2.5)],
  character: [Main.character("a"), Main.character("b")],
  escapedDoubleQuote: Main.escapedDoubleQuote,
  matchesEscapedDoubleQuote: Main.matchesEscapedDoubleQuote(Main.escapedDoubleQuote),
  rejectsOtherCharacter: Main.matchesEscapedDoubleQuote("x"),
  string: [Main.string("alexandrite"), Main.string("other")],
  boolean: [Main.boolean(true), Main.boolean(false)],
};
const expected = {
  integer: [true, false],
  number: [true, false],
  character: [true, false],
  escapedDoubleQuote: '"',
  matchesEscapedDoubleQuote: true,
  rejectsOtherCharacter: false,
  string: [true, false],
  boolean: [true, false],
};

deepStrictEqual(actual, expected, "unexpected literal pattern behavior");
