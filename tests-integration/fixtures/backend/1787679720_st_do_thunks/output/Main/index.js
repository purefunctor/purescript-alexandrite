import * as Data_Unit from "../Data.Unit/index.js";
import * as $foreign from "./foreign.js";
export function chained(seed) {
  const $action = constructST("first")(seed);
  return () => {
    const first = $action();
    const $action$1 = constructST("second")({ first });
    const second = $action$1();
    return constructST("third")({
      first,
      second
    })();
  };
}
export function discarded(seed) {
  const $action = constructST("discard-first")(Data_Unit.Unit);
  return () => {
    const $unit = $action();
    const result = mark("discard-let")(seed);
    return constructST("discard-second")(result)();
  };
}
export function pureAfterBind(seed) {
  const $action = constructST("pure-action")(seed);
  return () => {
    const value = $action();
    const $value = mark("pure-body")({ value });
    return $value;
  };
}
export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
