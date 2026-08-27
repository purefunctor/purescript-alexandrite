import * as Lookalike from "../Lookalike/index.js";
import * as Safe_Coerce from "../Safe.Coerce/index.js";
export const unsafelyCoerced = 42 | 0;
export const safelyCoerced = /* @__PURE__ */ Safe_Coerce.coerce({})(42 | 0);
export const lookalikeUnsafeCoerce = Lookalike.unsafeCoerce(42 | 0);
export const lookalikeSafeCoerce = Lookalike.coerce(42 | 0);
