import * as Data_Ring from "../Data.Ring/index.js";
import * as Data_Semiring from "../Data.Semiring/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";
export function booleanNot(value) {
  return !value;
}
export function integerAdd(left) {
  return (right) => {
    return left + right | 0;
  };
}
export function inlineIntegerAdd(left) {
  return (right) => {
    return left + right | 0;
  };
}
export function integerSubtract(left) {
  return (right) => {
    return left - right | 0;
  };
}
export function integerMultiply(left) {
  return (right) => {
    return left * right | 0;
  };
}
export function integerNegate(value) {
  return /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringInt)(value);
}
export function numberNegate(value) {
  return /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringNumber)(value);
}
export function genericNegate(ringValueDict) {
  return (value) => /* @__PURE__ */ Data_Ring.negate(ringValueDict)(value);
}
export function integerAddOrder($boolean) {
  return observe("left")(20 | 0) + observe("right")(22 | 0) | 0;
}
export function lookalikeAdd(left) {
  return (right) => {
    return /* @__PURE__ */ Lookalike.add(Lookalike.semiringInt)(left)(right);
  };
}
export function lookalikeNegate(value) {
  return Lookalike.negate(value);
}
export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];
export const integerNegateLiteral = /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringInt)(20 | 0);
export const inlineIntegerNegateLiteral = /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringInt)(20 | 0);
export const numberNegateLiteral = /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringNumber)(20.5);
export const numberNegateZero = /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringNumber)(0);
export const partiallyAppliedNegate = /* @__PURE__ */ Data_Ring.negate(Data_Ring.ringInt);
export const partiallyAppliedAdd = /* @__PURE__ */ Data_Semiring.add(Data_Semiring.semiringInt)(1 | 0);
