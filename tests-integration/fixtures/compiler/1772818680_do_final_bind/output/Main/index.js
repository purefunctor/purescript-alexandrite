import * as Control_Applicative from "../Control.Applicative/index.js";
function test_(applicativeDict) {
  return /* @__PURE__ */ Control_Applicative.pure(applicativeDict)(1 | 0);
}
export const test = /* @__PURE__ */ (() => {
  return () => {
    return 1 | 0;
  };
})();
export { test_ as "test'" };
