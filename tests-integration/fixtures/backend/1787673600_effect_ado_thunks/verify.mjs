import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

resetTrace();
const timedAdo = Main.timedAdo("ado");
const timedAdoConstruction = readTrace();
const timedAdoValue = timedAdo();
const timedAdoTrace = readTrace();

resetTrace();
const mapped = Main.mapped("mapped");
const mappedConstruction = readTrace();
const mappedValue = mapped();
const mappedTrace = readTrace();

resetTrace();
const applied = Main.applied("applied");
const appliedConstruction = readTrace();
const appliedValue = applied();
const appliedTrace = readTrace();

resetTrace();
const capturedPure = Main.capturedPure("pure");
const capturedPureConstruction = readTrace();
const capturedPureFirst = capturedPure();
const capturedPureSecond = capturedPure();
const capturedPureTrace = readTrace();

const actual = {
  timedAdoConstruction,
  timedAdoValue,
  timedAdoTrace,
  mappedConstruction,
  mappedValue,
  mappedTrace,
  appliedConstruction,
  appliedValue,
  appliedTrace,
  capturedPureConstruction,
  capturedPureFirst,
  capturedPureSecond,
  capturedPureTrace,
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
  mappedConstruction: ["mark:map-function", "construct:map-action"],
  mappedValue: "mapped",
  mappedTrace: [
    "mark:map-function",
    "construct:map-action",
    "run:map-action",
  ],
  appliedConstruction: [
    "construct:apply-function-action",
    "construct:apply-value-action",
  ],
  appliedValue: "applied",
  appliedTrace: [
    "construct:apply-function-action",
    "construct:apply-value-action",
    "run:apply-function-action",
    "run:apply-value-action",
  ],
  capturedPureConstruction: ["mark:pure-value"],
  capturedPureFirst: "pure",
  capturedPureSecond: "pure",
  capturedPureTrace: ["mark:pure-value"],
};

deepStrictEqual(actual, expected);
