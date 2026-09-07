import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Bind from "../Control.Bind/index.js";
function test_(bindDict) {
  return (applicativeDict) => {
    return /* @__PURE__ */ Control_Bind.bind(bindDict)(/* @__PURE__ */ Control_Applicative.pure(applicativeDict)(1 | 0))((x) => /* @__PURE__ */ Control_Applicative.pure(applicativeDict)(x));
  };
}
export const test = /* @__PURE__ */ (() => {
  return () => {
    return 1 | 0;
  };
})();
export { test_ as "test'" };
