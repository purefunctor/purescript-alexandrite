export const Box = ($value0) => ({
  tag: "Box",
  _1: $value0
});
export function method(dictionary) {
  return dictionary.method;
}
export function routeCase(routeRouteDict) {
  const $scrutinee = /* @__PURE__ */ method(routeRouteDict);
  if ($scrutinee.tag === "Box") {
    const { _1: value } = $scrutinee;
    return value;
  }
  throw new Error("Pattern match failure");
}
export function routeLet(routeRouteDict) {
  const $scrutinee = /* @__PURE__ */ method(routeRouteDict);
  if ($scrutinee.tag === "Box") {
    const { _1: value } = $scrutinee;
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
}
export function routeGuard(routeRouteDict) {
  {
    const $scrutinee = /* @__PURE__ */ method(routeRouteDict);
    if ($scrutinee.tag === "Box") {
      const { _1: value } = $scrutinee;
      return value;
    }
  }
  return /* @__PURE__ */ routeCase(routeRouteDict);
}
