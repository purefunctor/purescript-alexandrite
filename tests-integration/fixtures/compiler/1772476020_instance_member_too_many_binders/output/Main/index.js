import * as $runtime from "../runtime.js";
export function show(dictionary) {
  return dictionary.show;
}
const $lazy_showInt = $runtime.binding("showInt", () => {
  return { show: (x) => (y) => /* @__PURE__ */ show($lazy_showInt())(x) };
});
export const showInt = $lazy_showInt();
