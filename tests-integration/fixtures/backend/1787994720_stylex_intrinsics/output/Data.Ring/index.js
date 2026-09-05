import * as Data_Semiring from "../Data.Semiring/index.js";
import * as $foreign from "./foreign.js";
export function sub(dictionary) {
  return dictionary.sub;
}
export function negate(ringValueDict) {
  return (value) => /* @__PURE__ */ sub(ringValueDict)(/* @__PURE__ */ Data_Semiring.zero(/* @__PURE__ */ ringValueDict.Semiring0()))(value);
}
export const intSub = $foreign["intSub"];
export const numSub = $foreign["numSub"];
export const ringInt = {
  Semiring0: () => Data_Semiring.semiringInt,
  sub: intSub
};
export const ringNumber = {
  Semiring0: () => Data_Semiring.semiringNumber,
  sub: numSub
};
