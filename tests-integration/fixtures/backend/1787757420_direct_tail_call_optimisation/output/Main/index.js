import * as $foreign from "./foreign.js";
export function tailAccumulator(value) {
  return (accumulator) => {
    let $argument0 = value;
    let $argument1 = accumulator;
    while (true) {
      const $currentArgument0 = $argument0;
      const $currentArgument1 = $argument1;
      if (equalInt($currentArgument0)(0 | 0)) {
        return $currentArgument1;
      } else {
        $argument0 = decrementInt($currentArgument0);
        $argument1 = incrementInt($currentArgument1);
        continue;
      }
    }
  };
}
export function rotateArguments(iterations) {
  return (left) => {
    return (right) => {
      let $argument0 = iterations;
      let $argument1 = left;
      let $argument2 = right;
      while (true) {
        const $currentArgument0 = $argument0;
        const $currentArgument1 = $argument1;
        const $currentArgument2 = $argument2;
        if (equalInt($currentArgument0)(0 | 0)) {
          return $currentArgument1;
        } else {
          $argument0 = decrementInt($currentArgument0);
          $argument1 = $currentArgument2;
          $argument2 = $currentArgument1;
          continue;
        }
      }
    };
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const incrementInt = $foreign["incrementInt"];
