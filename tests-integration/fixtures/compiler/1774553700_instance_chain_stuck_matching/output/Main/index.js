import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const Spec = "Spec";
export function it(convertTGDict) {
  return ($string) => ($t) => "Spec";
}
export const action = $foreign["action"];
export const convertFunction = {};
export const convertAction = {};
export const test = /* @__PURE__ */ it(convertAction)("do")(action);
export const test2 = /* @__PURE__ */ it(convertFunction)("function")(($int) => action);
export const test3 = /* @__PURE__ */ it(convertAction)("variable")(action);
