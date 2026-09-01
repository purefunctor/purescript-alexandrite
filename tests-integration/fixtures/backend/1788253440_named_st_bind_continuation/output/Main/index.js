import * as Control_Bind from "../Control.Bind/index.js";
import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
export function namedContinuation($unit) {
  return () => {
    return 42 | 0;
  };
}
export function namedBind($unit) {
  const $function = Control_Bind.bind(Control_Monad_ST_Internal.bindST);
  const $effect = () => {
    return "Unit";
  };
  return /* @__PURE__ */ $function($effect)(namedContinuation);
}
export function runNamedBind($unit) {
  return Control_Monad_ST_Internal.run(namedBind("Unit"));
}
