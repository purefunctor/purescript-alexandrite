import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

const scenarios = [
  ["branch then", () => Main.branchResult(true)("branch"), "branch", ["branch-then", "branch-after"]],
  ["branch else", () => Main.branchResult(false)("branch"), "branch", ["branch-else", "branch-after"]],
  ["case first", () => Main.caseResult(Main.First)("case"), "case", ["case-first", "case-after"]],
  ["case second", () => Main.caseResult(Main.Second)("case"), "case", ["case-second", "case-after"]],
  ["guard true", () => Main.guardResult(true)("guard"), "guard", ["guard-true", "guard-after"]],
  ["guard false", () => Main.guardResult(false)("guard"), "guard", ["guard-false", "guard-after"]],
];

for (const [name, makeEffect, expectedValue, labels] of scenarios) {
  resetTrace();
  const effect = makeEffect();
  deepStrictEqual(readTrace(), [`construct:${labels[0]}`], `${name} construction`);
  deepStrictEqual(effect(), expectedValue, `${name} result`);
  const expectedTrace = labels.flatMap(label => [`construct:${label}`, `run:${label}`]);
  deepStrictEqual(readTrace(), expectedTrace, `${name} sequencing`);
}
