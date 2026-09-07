export const Box = ($value0) => ({
  tag: "Box",
  _1: $value0
});
export function show(dictionary) {
  return dictionary.show;
}
export const showBox = /* @__PURE__ */ (() => {
  const $closure = ($box) => {
    if ($box.tag === "Box") {
      const { _1: x } = $box;
      let $result;
      throw new Error("Generated code reached a source error");
      return /* @__PURE__ */ show($result)(x);
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return { show: $closure };
})();
