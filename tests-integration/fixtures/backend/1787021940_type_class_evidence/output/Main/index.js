import * as $foreign from "./foreign.js";

function genericEqual$closure(dictionary0) {
  return left => {
    return right => {
      return dictionary0.equal(left)(right);
    };
  };
}

function superclassEqual$closure(dictionary1) {
  return left => {
    return right => {
      return dictionary1.superclass17.equal(left)(right);
    };
  };
}

export function genericEqual(dictionary0) {
  return genericEqual$closure(dictionary0);
}

export function superclassEqual(dictionary1) {
  return superclassEqual$closure(dictionary1);
}

export const equalInt = $foreign["equalInt"];
export const lessThanInt = $foreign["lessThanInt"];

export const equalInt1 = { equal: equalInt };

export const orderedInt = { superclass17: equalInt1, lessThan: lessThanInt };

export const concreteEqual = equalInt1.equal(1 | 0)(2 | 0);

export const concreteLessThan = orderedInt.lessThan(1 | 0)(2 | 0);
