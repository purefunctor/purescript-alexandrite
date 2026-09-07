import * as $foreign from "./foreign.js";
export function uncurriedTail(value, accumulator) {
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
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const incrementInt = $foreign["incrementInt"];
