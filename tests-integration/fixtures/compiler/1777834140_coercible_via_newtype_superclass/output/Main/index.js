import * as Data_Newtype from "../Data.Newtype/index.js";
import * as Safe_Coerce from "../Safe.Coerce/index.js";
export function useUnwrap(newtypeTHiddenIntDict) {
  return /* @__PURE__ */ Data_Newtype.unwrap(newtypeTHiddenIntDict);
}
export function useCoerce(newtypeTHiddenIntDict) {
  return /* @__PURE__ */ Safe_Coerce.coerce(/* @__PURE__ */ newtypeTHiddenIntDict.Coercible0());
}
