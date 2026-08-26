import * as $foreign from "./foreign.js";
export function uncurriedTail(value, accumulator) {
  if (equalInt(value)(0 | 0)) {
    return accumulator;
  } else {
    return uncurriedTail(decrementInt(value), incrementInt(accumulator));
  }
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const incrementInt = $foreign["incrementInt"];
