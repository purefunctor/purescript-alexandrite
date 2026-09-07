import * as Data_Unit from "../Data.Unit/index.js";
import * as $foreign from "./foreign.js";
export function localBind($unit) {
  const continuation = makeContinuation(Data_Unit.unit);
  const $value = Data_Unit.unit;
  return () => {
    return continuation($value)();
  };
}
export const makeContinuation = $foreign["makeContinuation"];
