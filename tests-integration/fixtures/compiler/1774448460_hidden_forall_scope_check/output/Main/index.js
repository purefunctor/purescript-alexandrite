export const Unit = "Unit";
export const Respond = ($value0) => ($value1) => ({
  tag: "Respond",
  _1: $value0,
  _2: $value1
});
export const Pure = ($value0) => ({
  tag: "Pure",
  _1: $value0
});
export function respond(monadMDict) {
  return (a) => ({
    tag: "Respond",
    _1: a,
    _2: Pure
  });
}
function $yield(monadMDict) {
  return /* @__PURE__ */ respond(monadMDict);
}
export { $yield as "yield" };
