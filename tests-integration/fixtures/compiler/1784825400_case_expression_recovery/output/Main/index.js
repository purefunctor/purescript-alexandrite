export function mismatchedBinders(first) {
  return (second) => {
    return true;
  };
}
export function missingResult(value) {
  throw new Error("Generated code reached a source error");
}
export function missingGuardResult(value) {
  {
    if (true) {
      throw new Error("Generated code reached a source error");
    }
  }
  throw new Error("Pattern match failure");
}
export function missingBranchBody(value) {
  throw new Error("Generated code reached a source error");
}
export const missingScrutinee = /* @__PURE__ */ (() => {
  return true;
})();
