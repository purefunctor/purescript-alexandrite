import * as $foreign from "./foreign.js";
export function mutualEven(value) {
  if (equalInt(value)(0 | 0)) {
    return true;
  } else {
    return mutualOdd(decrementInt(value));
  }
}
export function mutualOdd(value) {
  if (equalInt(value)(0 | 0)) {
    return false;
  } else {
    return mutualEven(decrementInt(value));
  }
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
