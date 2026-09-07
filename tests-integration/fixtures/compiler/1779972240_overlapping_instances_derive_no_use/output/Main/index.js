export const Box = "Box";
export const eqBox1 = /* @__PURE__ */ (() => {
  const $closure = (left) => {
    return (right) => {
      if (left === "Box" && right === "Box") {
        return true;
      }
      throw new Error("Pattern match failure");
    };
  };
  return { eq: $closure };
})();
export const eqBox = { eq: ($box) => ($box$1) => true };
