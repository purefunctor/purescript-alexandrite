import * as $foreign from "./foreign.js";
function $tail_singleArgument_twoArguments($state, $argument0, $argument1) {
  while (true) {
    switch ($state) {
      // singleArgument
      case 0: {
        const $currentArgument0 = $argument0;
        if (equalInt($currentArgument0)(0 | 0)) {
          return $currentArgument0;
        } else {
          $argument0 = decrementInt($currentArgument0);
          $argument1 = 0 | 0;
          $state = 1;
          continue;
        }
      }
      // twoArguments
      case 1: {
        const $currentArgument0$1 = $argument0;
        const $currentArgument1 = $argument1;
        if (equalInt($currentArgument0$1)(0 | 0)) {
          return $currentArgument1;
        } else {
          $argument0 = decrementInt($currentArgument0$1);
          $argument1 = null;
          $state = 0;
          continue;
        }
      }
    }
  }
}
export function singleArgument(value) {
  return $tail_singleArgument_twoArguments(0, value, null);
}
export function twoArguments(value$1) {
  return (accumulator) => {
    return $tail_singleArgument_twoArguments(1, value$1, accumulator);
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
