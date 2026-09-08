import * as Data_Unit from "../Data.Unit/index.js";
import * as $foreign from "./foreign.js";
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
export function tailBind(value) {
  const $tail_tailBind = ($state, $argument0) => {
    while (true) {
      const $currentArgument0 = $argument0;
      if (equalInt($currentArgument0)(0 | 0)) {
        return () => {
          return [false, $currentArgument0];
        };
      } else {
        const $value = Data_Unit.unit;
        return () => {
          const $tailArgument = decrementInt($currentArgument0);
          return [
            true,
            0,
            $tailArgument
          ];
        };
      }
    }
  };
  const $initialStep = $tail_tailBind(0, value);
  return () => {
    let $step;
    $step = $initialStep;
    while (true) {
      const $result = $step();
      if (!$result[0]) {
        return $result[1];
      }
      $step = $tail_tailBind($result[1], $result[2]);
    }
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
