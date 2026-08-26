import * as $foreign from "./foreign.js";
export function localTail(value) {
  const go = (current) => {
    if (equalInt(current)(0 | 0)) {
      return current;
    } else {
      return go(decrementInt(current));
    }
  };
  return go(value);
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
