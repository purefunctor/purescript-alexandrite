import * as Data_HeytingAlgebra from "../Data.HeytingAlgebra/index.js";
import * as Data_Ring from "../Data.Ring/index.js";
import * as Data_Semiring from "../Data.Semiring/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";

export function booleanNot(value) {
  return Data_HeytingAlgebra.heytingAlgebraBoolean.not(value);
}

export function integerAdd(left) {
  return right => {
    return Data_Semiring.semiringInt.add(left)(right);
  };
}

export function integerSubtract(left) {
  return right => {
    return Data_Ring.ringInt.sub(left)(right);
  };
}

export function integerMultiply(left) {
  return right => {
    return Data_Semiring.semiringInt.mul(left)(right);
  };
}

export function integerNegate(value) {
  return Data_Ring.ringInt.negate(value);
}

export function integerAddOrder($boolean) {
  return Data_Semiring.semiringInt.add(observe("left")(20 | 0))(observe("right")(22 | 0));
}

export function lookalikeAdd(left) {
  return right => {
    return Lookalike.semiringInt.add(left)(right);
  };
}

export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];

export const partiallyAppliedAdd = Data_Semiring.semiringInt.add(1 | 0);
