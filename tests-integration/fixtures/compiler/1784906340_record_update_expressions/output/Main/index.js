export function identity(value) {
  return value;
}
export const record = {
  first: 1 | 0,
  nested: { value: 2 | 0 },
  transform: identity
};
export const leaf = /* @__PURE__ */ (() => {
  return {
    ...record,
    first: 2 | 0
  };
})();
export const typeChanging = /* @__PURE__ */ (() => {
  return {
    ...record,
    first: "two"
  };
})();
export const multiple = /* @__PURE__ */ (() => {
  return {
    ...record,
    first: 2 | 0,
    transform: identity
  };
})();
export const nested = /* @__PURE__ */ (() => {
  return {
    ...record,
    nested: {
      ...record.nested,
      value: 3 | 0
    }
  };
})();
export const accessValue = /* @__PURE__ */ (() => {
  return {
    ...record,
    first: record.nested.value
  };
})();
export const applicationBase = /* @__PURE__ */ (() => {
  return {
    ...identity(record),
    first: 2 | 0
  };
})();
export const argument = /* @__PURE__ */ (() => {
  return identity({
    ...record,
    first: 2 | 0
  });
})();
