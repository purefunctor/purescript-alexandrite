import * as Data_Function from "../Data.Function/index.js";

export function unwrap(value) {
  return value;
}

export function eqWrapper(eqADict) {
  return { eq: Data_Function.on(eqADict.eq)(unwrap) };
}
