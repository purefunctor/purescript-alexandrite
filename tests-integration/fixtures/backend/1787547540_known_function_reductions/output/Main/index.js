import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";

export function directApplyOrder($boolean) {
  return observe("function")(value => value)(observe("argument")(42 | 0));
}

export function flippedApplyOrder($boolean) {
  const call$1 = observe("argument")(42 | 0);
  return observe("function")(value => value)(call$1);
}

export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];

export const directApply = (() => {
  return (value => value)(42 | 0);
})();

export const flippedApply = (() => {
  return (value => value)(42 | 0);
})();

export const functionIdentity = 42 | 0;

export const coerced = 42 | 0;

export const lookalikeApply = (() => {
  return Lookalike.apply(value => value)(42 | 0);
})();

export const lookalikeIdentity = Lookalike.identity(Lookalike.categoryFn)(42 | 0);

export const lookalikeCoerce = Lookalike.unsafeCoerce(42 | 0);
