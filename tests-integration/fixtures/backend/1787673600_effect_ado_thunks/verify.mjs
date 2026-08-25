import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

resetTrace();
const timedAdo = Main.timedAdo("ado");
const timedAdoConstruction = readTrace();
const timedAdoValue = timedAdo();
const timedAdoTrace = readTrace();

const actual = {
  timedAdoConstruction,
  timedAdoValue,
  timedAdoTrace,
};

const expected = {
  timedAdoConstruction: ["construct:ado-first", "construct:ado-second"],
  timedAdoValue: { first: "ado", second: { seed: "ado" } },
  timedAdoTrace: [
    "construct:ado-first",
    "construct:ado-second",
    "run:ado-first",
    "run:ado-second",
  ],
};

deepStrictEqual(actual, expected);
