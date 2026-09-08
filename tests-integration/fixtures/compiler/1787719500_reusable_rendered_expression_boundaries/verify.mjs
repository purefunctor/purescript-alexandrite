import { deepStrictEqual, strictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

resetTrace();
strictEqual(Main.stableApplication(value => value + 1)(true), 2);
deepStrictEqual(readTrace(), ["observe:application-then"]);

resetTrace();
strictEqual(Main.observedApplication(true), 3);
deepStrictEqual(readTrace(), ["read:apply", "observe:observed-then", "apply"]);

resetTrace();
deepStrictEqual(Main.stableArray(14)(false), [14, 6]);
deepStrictEqual(readTrace(), ["observe:array-else"]);

resetTrace();
deepStrictEqual(Main.observedArray(true), [13, 7]);
deepStrictEqual(readTrace(), ["read:value", "observe:observed-array-then"]);

resetTrace();
const pureEffect = Main.stablePure(15);
deepStrictEqual(readTrace(), []);
strictEqual(pureEffect(), 15);
deepStrictEqual(readTrace(), []);

resetTrace();
const mappedEffect = Main.stableMap(Main.observe("stable-map-result"));
deepStrictEqual(readTrace(), ["construct:stable-map"]);
strictEqual(mappedEffect(), 9);
deepStrictEqual(readTrace(), [
  "construct:stable-map",
  "run:stable-map",
  "observe:stable-map-result",
]);

resetTrace();
const observedEffect = Main.observedMap(false);
deepStrictEqual(readTrace(), ["read:apply", "construct:observed-map"]);
strictEqual(observedEffect(), 10);
deepStrictEqual(readTrace(), [
  "read:apply",
  "construct:observed-map",
  "run:observed-map",
  "apply",
]);

resetTrace();
const mixedEffect = Main.mixedApply(false);
deepStrictEqual(readTrace(), [
  "construct:mixed-function",
  "construct:mixed-argument-first",
]);
strictEqual(mixedEffect(), 18);
deepStrictEqual(readTrace(), [
  "construct:mixed-function",
  "construct:mixed-argument-first",
  "run:mixed-function",
  "run:mixed-argument-first",
  "construct:mixed-argument-second",
  "run:mixed-argument-second",
  "observe:mixed-call",
]);

resetTrace();
const [joinedEffect] = Main.joinedEffect(true);
deepStrictEqual(readTrace(), ["read:apply", "construct:joined-then"]);
strictEqual(joinedEffect(), 16);
deepStrictEqual(readTrace(), [
  "read:apply",
  "construct:joined-then",
  "run:joined-then",
  "apply",
]);

resetTrace();
strictEqual(Main.joinedPattern(false), 12);
deepStrictEqual(readTrace(), ["observe:pattern-else"]);
