export const Representation = "Representation";
export function parent(dictionary) {
  return dictionary.parent;
}
export function child(dictionary) {
  return dictionary.child;
}
export function parentWrapper(gateDict) {
  return { parent: (value) => value };
}
export function parentFromChild(childValueDict) {
  return /* @__PURE__ */ parent(/* @__PURE__ */ childValueDict.Parent0());
}
export const parentRepresentation = { parent: (value) => value };
export const childRepresentation = {
  Parent0: () => parentRepresentation,
  child: (value) => value
};
export const childWrapper = childRepresentation;
export const useDelegateSuperclass = /* @__PURE__ */ parentFromChild(childWrapper);
