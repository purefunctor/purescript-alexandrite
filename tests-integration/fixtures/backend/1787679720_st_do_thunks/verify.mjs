import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

resetTrace();
const chained = Main.chained("seed");
const chainedConstruction = readTrace();
const chainedFirst = chained();
const chainedFirstTrace = readTrace();
const chainedSecond = chained();
const chainedSecondTrace = readTrace();

resetTrace();
const discarded = Main.discarded("discarded");
const discardedConstruction = readTrace();
const discardedValue = discarded();
const discardedTrace = readTrace();

resetTrace();
const pureAfterBind = Main.pureAfterBind("captured");
const pureAfterBindConstruction = readTrace();
const pureAfterBindValue = pureAfterBind();
const pureAfterBindTrace = readTrace();

const actual = {
  chainedConstruction,
  chainedFirst,
  chainedFirstTrace,
  chainedSecond,
  chainedSecondTrace,
  discardedConstruction,
  discardedValue,
  discardedTrace,
  pureAfterBindConstruction,
  pureAfterBindValue,
  pureAfterBindTrace,
};

const expected = {
  chainedConstruction: ["construct:first"],
  chainedFirst: { first: "seed", second: { first: "seed" } },
  chainedFirstTrace: [
    "construct:first",
    "run:first",
    "construct:second",
    "run:second",
    "construct:third",
    "run:third",
  ],
  chainedSecond: { first: "seed", second: { first: "seed" } },
  chainedSecondTrace: [
    "construct:first",
    "run:first",
    "construct:second",
    "run:second",
    "construct:third",
    "run:third",
    "run:first",
    "construct:second",
    "run:second",
    "construct:third",
    "run:third",
  ],
  discardedConstruction: ["construct:discard-first"],
  discardedValue: "discarded",
  discardedTrace: [
    "construct:discard-first",
    "run:discard-first",
    "mark:discard-let",
    "construct:discard-second",
    "run:discard-second",
  ],
  pureAfterBindConstruction: ["construct:pure-action"],
  pureAfterBindValue: { value: "captured" },
  pureAfterBindTrace: [
    "construct:pure-action",
    "run:pure-action",
    "mark:pure-body",
  ],
};

deepStrictEqual(actual, expected);
