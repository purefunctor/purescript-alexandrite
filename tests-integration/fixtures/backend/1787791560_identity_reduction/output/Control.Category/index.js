import * as Control_Semigroupoid from "../Control.Semigroupoid/index.js";
export function identity(dictionary) {
  return dictionary.identity;
}
export const categoryFn = {
  Semigroupoid0: () => Control_Semigroupoid.semigroupoidFn,
  identity: (value) => value
};
