import * as $foreign from "./foreign.js";

export function genericEqual(equalADict) {
  function genericEqual$closure(equalADict) {
    return left => {
      return right => {
        return equalADict.equal(left)(right);
      };
    };
  }
  return genericEqual$closure(equalADict);
}

export function superclassEqual(orderedADict) {
  function superclassEqual$closure(orderedADict) {
    return left => {
      return right => {
        return (orderedADict.Equal0({})).equal(left)(right);
      };
    };
  }
  return superclassEqual$closure(orderedADict);
}

export const equalInt = $foreign["equalInt"];
export const lessThanInt = $foreign["lessThanInt"];

export const equalInt1 = { equal: equalInt };

export const orderedInt = (() => {
  function orderedInt$initialize$closure(unit) {
    return equalInt1;
  }
  return { Equal0: orderedInt$initialize$closure, lessThan: lessThanInt };
})();

export const concreteEqual = equalInt1.equal(1 | 0)(2 | 0);

export const concreteLessThan = orderedInt.lessThan(1 | 0)(2 | 0);
