export const Proxy = "Proxy";
export function use(dictionary) {
  return dictionary.use;
}
export function use1(rowToListRowXsDict) {
  return { use: "Proxy" };
}
export const test = /* @__PURE__ */ use(/* @__PURE__ */ use1({}));
