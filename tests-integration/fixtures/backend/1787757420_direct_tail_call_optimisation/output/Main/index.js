import * as $foreign from "./foreign.js";
export function tailAccumulator(value) {
  return (accumulator) => {
    if (equalInt(value)(0 | 0)) {
      return accumulator;
    } else {
      return tailAccumulator(decrementInt(value))(incrementInt(accumulator));
    }
  };
}
export function rotateArguments(iterations) {
  return (left) => {
    return (right) => {
      if (equalInt(iterations)(0 | 0)) {
        return left;
      } else {
        return rotateArguments(decrementInt(iterations))(right)(left);
      }
    };
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const incrementInt = $foreign["incrementInt"];
