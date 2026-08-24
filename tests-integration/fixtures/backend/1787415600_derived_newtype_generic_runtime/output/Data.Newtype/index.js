import * as Safe_Coerce from "../Safe.Coerce/index.js";

export function wrap(newtypeTADict) {
  return Safe_Coerce.coerce({});
}

export function unwrap(newtypeTADict) {
  return Safe_Coerce.coerce(newtypeTADict.Coercible0());
}
