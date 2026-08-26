export function first($boolean) {
  if ($boolean === true) {
    return 1 | 0;
  }
  if ($boolean === false) {
    return second(true);
  }
  throw new Error("Pattern match failure");
}
export function second($boolean) {
  if ($boolean === true) {
    return 2 | 0;
  }
  if ($boolean === false) {
    return first(true);
  }
  throw new Error("Pattern match failure");
}
export const later = 42 | 0;
export const forward = later;
export const recursiveValue = /* @__PURE__ */ (() => {
  const $closure = (value) => {
    if (value) {
      return 3 | 0;
    } else {
      return recursivePeer.run(true);
    }
  };
  return { run: $closure };
})();
export const recursivePeer = /* @__PURE__ */ (() => {
  const $closure = (value) => {
    if (value) {
      return 4 | 0;
    } else {
      return recursiveValue.run(true);
    }
  };
  return { run: $closure };
})();
