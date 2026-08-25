import * as Data_Functor from "../Data.Functor/index.js";

export function apply(dictionary) {
  return dictionary.apply;
}

export const applyFn = { Functor0: () => Data_Functor.functorFn, apply: f => g => x => f(x)(g(x)) };
