import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";
export function directApplyOrder($boolean) {
  return observe("function")((value) => value)(observe("argument")(42 | 0));
}
export function flippedApplyOrder($boolean) {
  const applyArgument = observe("argument")(42 | 0);
  const applyFunction = observe("function")((value) => value);
  return applyFunction(applyArgument);
}
export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];
export const directApply = ((value) => value)(42 | 0);
export const flippedApply = ((value) => value)(42 | 0);
export const functionIdentity = 42 | 0;
export const coerced = 42 | 0;
export const lookalikeApply = Lookalike.apply((value) => value)(42 | 0);
export const lookalikeIdentity = /* @__PURE__ */ Lookalike.identity(Lookalike.categoryFn)(42 | 0);
export const lookalikeCoerce = Lookalike.unsafeCoerce(42 | 0);
