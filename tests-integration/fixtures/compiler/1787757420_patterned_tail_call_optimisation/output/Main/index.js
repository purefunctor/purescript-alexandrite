import * as $foreign from "./foreign.js";
export function patternedTail($record) {
  let $argument0 = $record;
  while (true) {
    const $currentArgument0 = $argument0;
    const value = $currentArgument0.value;
    if (equalInt(value)(0 | 0)) {
      return value;
    } else {
      $argument0 = { value: decrementInt(value) };
      continue;
    }
  }
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
