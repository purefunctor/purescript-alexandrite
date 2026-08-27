import * as $foreign from "./foreign.js";
export function localTail(value) {
  const go = (current) => {
    let $argument0 = current;
    while (true) {
      const $currentArgument0 = $argument0;
      if (equalInt($currentArgument0)(0 | 0)) {
        return $currentArgument0;
      } else {
        $argument0 = decrementInt($currentArgument0);
        continue;
      }
    }
  };
  return go(value);
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
