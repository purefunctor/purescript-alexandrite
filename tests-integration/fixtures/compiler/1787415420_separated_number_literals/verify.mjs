import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";

const actual = {
  separatedNumber: Main.separatedNumber,
  separatedNumberParts: Main.separatedNumberParts,
  separatedExponent: Main.separatedExponent,
  separatedUpperExponent: Main.separatedUpperExponent,
  separatedNegativeExponent: Main.separatedNegativeExponent,
  matchesSeparatedNumber: Main.matchesSeparatedNumber(1005),
  matchesSeparatedExponent: Main.matchesSeparatedNumber(1200),
  matchesSeparatedUpperExponent: Main.matchesSeparatedNumber(1300),
  matchesSeparatedNegativeExponent: Main.matchesSeparatedNumber(14),
  matchesNegativeSeparatedNumber: Main.matchesSeparatedNumber(-15),
  rejectsDifferentNumber: Main.matchesSeparatedNumber(1006),
};
const expected = {
  separatedNumber: 4294967295,
  separatedNumberParts: 1234,
  separatedExponent: 1200,
  separatedUpperExponent: 1300,
  separatedNegativeExponent: 14,
  matchesSeparatedNumber: true,
  matchesSeparatedExponent: true,
  matchesSeparatedUpperExponent: true,
  matchesSeparatedNegativeExponent: true,
  matchesNegativeSeparatedNumber: true,
  rejectsDifferentNumber: false,
};

deepStrictEqual(actual, expected, "unexpected separated Number behavior");
