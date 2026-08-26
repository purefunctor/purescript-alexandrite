import * as $foreign from "./foreign.js";
export function effectTail(value) {
  if (equalInt(value)(0 | 0)) {
    return () => {
      return value;
    };
  } else {
    const $action = constructTick(value);
    return () => {
      const $unit = $action();
      return effectTail(decrementInt(value))();
    };
  }
}
export function effectMutualEven(value) {
  if (equalInt(value)(0 | 0)) {
    return () => {
      return true;
    };
  } else {
    const $action = constructTick(value);
    return () => {
      const $unit = $action();
      return effectMutualOdd(decrementInt(value))();
    };
  }
}
export function effectMutualOdd(value) {
  if (equalInt(value)(0 | 0)) {
    return () => {
      return false;
    };
  } else {
    const $action = constructTick(value);
    return () => {
      const $unit = $action();
      return effectMutualEven(decrementInt(value))();
    };
  }
}
export function effectMixedShort(value) {
  if (equalInt(value)(0 | 0)) {
    return () => {
      return value;
    };
  } else {
    const $action = constructTick(value);
    return () => {
      const $unit = $action();
      return effectMixedLong(decrementInt(value))(0 | 0)();
    };
  }
}
export function effectMixedLong(value) {
  return (accumulator) => {
    if (equalInt(value)(0 | 0)) {
      return () => {
        return accumulator;
      };
    } else {
      const $action = constructTick(value);
      return () => {
        const $unit = $action();
        return effectMixedShort(decrementInt(value))();
      };
    }
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const constructTick = $foreign["constructTick"];
