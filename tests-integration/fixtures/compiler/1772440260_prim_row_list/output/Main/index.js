export function rowToListSimple(rowToListRowListDict) {
  return "Proxy";
}
export function rowToListMultiple(rowToListRowListDict) {
  return "Proxy";
}
export function rowToListEmpty(rowToListRowListDict) {
  return "Proxy";
}
export function rowToListThree(rowToListRowListDict) {
  return "Proxy";
}
export function stuckOpenRow(rowToListRowListDict) {
  return ($proxy) => "Proxy";
}
export const forceSolve = {
  rowToListSimple: /* @__PURE__ */ rowToListSimple({}),
  rowToListMultiple: /* @__PURE__ */ rowToListMultiple({}),
  rowToListEmpty: /* @__PURE__ */ rowToListEmpty({}),
  rowToListThree: /* @__PURE__ */ rowToListThree({}),
  nowSolved: /* @__PURE__ */ stuckOpenRow({})("Proxy")
};
