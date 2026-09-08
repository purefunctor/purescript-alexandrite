import * as Partial_Unsafe from "../Partial.Unsafe/index.js";
export function safeZero(partialDict) {
  const $closure = ($int) => {
    if ($int === (0 | 0)) {
      return 0 | 0;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return $closure;
}
export const unsafeZeroLambda = /* @__PURE__ */ (() => {
  const $closure = (partialDict) => {
    const $closure$1 = ($int) => {
      if ($int === (0 | 0)) {
        return 0 | 0;
      } else {
        throw new Error("Pattern match failure");
      }
    };
    return $closure$1;
  };
  return Partial_Unsafe.unsafePartial($closure);
})();
export const unsafeZeroName = Partial_Unsafe.unsafePartial((partialDict) => /* @__PURE__ */ safeZero(partialDict));
