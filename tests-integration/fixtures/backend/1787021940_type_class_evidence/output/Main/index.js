import * as $foreign from "./foreign.js";

export function genericEqual(equalADict) {
  return left => right => equalADict.equal(left)(right);
}

export function superclassEqual(orderedADict) {
  return left => right => (orderedADict.Equal0()).equal(left)(right);
}

export const equalInt = $foreign["equalInt"];
export const lessThanInt = $foreign["lessThanInt"];

export const equalInt1 = { equal: equalInt };

export const orderedInt = (() => {
  return { Equal0: () => equalInt1, lessThan: lessThanInt };
})();

export const concreteEqual = equalInt1.equal(1 | 0)(2 | 0);

export const concreteLessThan = orderedInt.lessThan(1 | 0)(2 | 0);
