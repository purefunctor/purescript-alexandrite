export const Wrap = ($value0) => ({
  tag: "Wrap",
  _1: $value0
});
export function lift(dictionary) {
  return dictionary.lift;
}
export function render(dictionary) {
  return dictionary.render;
}
export function renderer(dictionary) {
  return dictionary.renderer;
}
export function liftPlain(dictionary) {
  return dictionary.liftPlain;
}
export const transformWrap = { lift: Wrap };
export const rendererString = { renderer: (renderADict) => /* @__PURE__ */ render(renderADict) };
export const transformPlainWrap = { liftPlain: (functorFDict) => Wrap };
