import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
import * as Data_Unit from "../Data.Unit/index.js";
export function namedContinuation($unit) {
  return () => {
    return 42 | 0;
  };
}
export function namedBind($unit) {
  const $value = Data_Unit.unit;
  return () => {
    return namedContinuation($value)();
  };
}
export function runNamedBind($unit) {
  return Control_Monad_ST_Internal.run(namedBind(Data_Unit.unit));
}
