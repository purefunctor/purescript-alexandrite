import * as Main from "./output/Main/index.js";
import { readTrace, resetTrace } from "./output/Main/foreign.js";

resetTrace();
const sequential = Main.sequential();
const sequentialTrace = readTrace();
resetTrace();
const independent = Main.independent();
const independentTrace = readTrace();
resetTrace();
const pure = Main.pureValue();
const actual = {
  sequential,
  sequentialTrace,
  independent,
  independentTrace,
  pure,
  pureTrace: readTrace(),
};
const expected = {
  sequential: "value:20",
  sequentialTrace: ["first", "second"],
  independent: { first: 20, second: true },
  independentTrace: ["first", "independent"],
  pure: 42,
  pureTrace: [],
};

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error(
    `unexpected effects\nactual: ${JSON.stringify(actual)}\nexpected: ${JSON.stringify(expected)}`,
  );
}
