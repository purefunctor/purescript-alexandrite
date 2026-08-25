import * as $foreign from "./foreign.js";

export function equal(dictionary) {
  return dictionary.equal;
}

export function lessThan(dictionary) {
  return dictionary.lessThan;
}

export function genericEqual(equalADict) {
  return left => right => equal(equalADict)(left)(right);
}

export function superclassEqual(orderedADict) {
  return left => right => equal(orderedADict.Equal0())(left)(right);
}

export const equalInt = $foreign["equalInt"];
export const lessThanInt = $foreign["lessThanInt"];

export const equalInt1 = { equal: equalInt };

export const orderedInt = (() => {
  return { Equal0: () => equalInt1, lessThan: lessThanInt };
})();

export const concreteEqual = equal(equalInt1)(1 | 0)(2 | 0);

export const concreteLessThan = lessThan(orderedInt)(1 | 0)(2 | 0);
