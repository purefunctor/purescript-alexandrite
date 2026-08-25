import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

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
