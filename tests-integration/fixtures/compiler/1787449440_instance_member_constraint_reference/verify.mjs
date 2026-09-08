import * as Main from "./output/Main/index.js";

const eqInt = { eq: left => right => left === right };
const eqWrapper = Main.eqWrapper(eqInt);

if (!eqWrapper.eq(1)(1)) {
  throw new Error("expected wrapped equal values to compare equal");
}
