import * as $foreign from "./foreign.js";
function $tail_mutualEven_mutualOdd($state, $argument0) {
  while (true) {
    switch ($state) {
      // mutualEven
      case 0: {
        const $currentArgument0 = $argument0;
        if (equalInt($currentArgument0)(0 | 0)) {
          return true;
        } else {
          $argument0 = decrementInt($currentArgument0);
          $state = 1;
          continue;
        }
      }
      // mutualOdd
      case 1: {
        const $currentArgument0$1 = $argument0;
        if (equalInt($currentArgument0$1)(0 | 0)) {
          return false;
        } else {
          $argument0 = decrementInt($currentArgument0$1);
          $state = 0;
          continue;
        }
      }
    }
  }
}
export function mutualEven(value) {
  return $tail_mutualEven_mutualOdd(0, value);
}
export function mutualOdd(value$1) {
  return $tail_mutualEven_mutualOdd(1, value$1);
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
