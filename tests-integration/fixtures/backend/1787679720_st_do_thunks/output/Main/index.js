import * as Data_Unit from "../Data.Unit/index.js";
import * as $foreign from "./foreign.js";

export function chained(seed) {
  const $action = constructST("first")(seed);
  const $effect = () => {
    const first = $action();
    const $action$1 = constructST("second")({ first: first });
    const second = $action$1();
    return constructST("third")({ first: first, second: second })();
  };
  return $effect;
}

export function discarded(seed) {
  const $action = constructST("discard-first")(Data_Unit.Unit);
  const $effect = () => {
    const $unit = $action();
    const result = mark("discard-let")(seed);
    return constructST("discard-second")(result)();
  };
  return $effect;
}

export function pureAfterBind(seed) {
  const $action = constructST("pure-action")(seed);
  const $effect = () => {
    const value = $action();
    const $value = mark("pure-body")({ value: value });
    return $value;
  };
  return $effect;
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
