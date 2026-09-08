import * as Data_Newtype from "../Data.Newtype/index.js";
export function test(x) {
  return /* @__PURE__ */ Data_Newtype.unwrap(newtypeEndo)(x);
}
export const newtypeEndo = { Coercible0: () => ({}) };
