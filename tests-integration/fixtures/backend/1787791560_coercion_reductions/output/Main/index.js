import * as Lookalike from "../Lookalike/index.js";
export const unsafelyCoerced = 42 | 0;
export const safelyCoerced = 42 | 0;
export const lookalikeUnsafeCoerce = Lookalike.unsafeCoerce(42 | 0);
export const lookalikeSafeCoerce = Lookalike.coerce(42 | 0);
