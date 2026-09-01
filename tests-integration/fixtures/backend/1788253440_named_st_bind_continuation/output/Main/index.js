import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
export function namedContinuation($unit) {
  return () => {
    return 42 | 0;
  };
}
export function namedBind($unit) {
  return () => {
    let bindValue;
    bindValue = "Unit";
    return namedContinuation(bindValue)();
  };
}
export function runNamedBind($unit) {
  return Control_Monad_ST_Internal.run(namedBind("Unit"));
}
