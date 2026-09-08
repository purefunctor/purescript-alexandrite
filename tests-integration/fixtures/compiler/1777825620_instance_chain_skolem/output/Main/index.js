export function step(dictionary) {
  return dictionary.step;
}
export function done(dictionary) {
  return dictionary.done;
}
export function walk(dictionary) {
  return dictionary.walk;
}
export function walkArg(stepFAXA_Dict) {
  return (walkFA_XsDict) => {
    return { walk: (f) => (a) => (x) => /* @__PURE__ */ walk(walkFA_XsDict)(f)(/* @__PURE__ */ step(stepFAXA_Dict)(f)(a)(x)) };
  };
}
export function walkBase(doneFAXDict) {
  return { walk: /* @__PURE__ */ done(doneFAXDict) };
}
