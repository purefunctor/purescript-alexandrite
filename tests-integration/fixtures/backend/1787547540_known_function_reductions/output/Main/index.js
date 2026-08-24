import * as Control_Category from "../Control.Category/index.js";
import * as Data_Function from "../Data.Function/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as Unsafe_Coerce from "../Unsafe.Coerce/index.js";
import * as $foreign from "./foreign.js";

export function directApplyOrder($boolean) {
  return Data_Function.apply(observe("function")(value => value))(observe("argument")(42 | 0));
}

export function flippedApplyOrder($boolean) {
  return Data_Function.applyFlipped(observe("argument")(42 | 0))(
    observe("function")(value => value)
  );
}

export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];

export const directApply = (() => {
  return Data_Function.apply(value => value)(42 | 0);
})();

export const flippedApply = (() => {
  return Data_Function.applyFlipped(42 | 0)(value => value);
})();

export const functionIdentity = Control_Category.categoryFn.identity(42 | 0);

export const coerced = Unsafe_Coerce.unsafeCoerce(42 | 0);

export const lookalikeApply = (() => {
  return Lookalike.apply(value => value)(42 | 0);
})();

export const lookalikeIdentity = Lookalike.categoryFn.identity(42 | 0);

export const lookalikeCoerce = Lookalike.unsafeCoerce(42 | 0);
