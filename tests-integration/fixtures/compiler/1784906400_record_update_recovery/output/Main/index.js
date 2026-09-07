export const record = {
  nested: { value: 1 | 0 },
  value: 2 | 0
};
export const missingValue = (() => {
  const $record = { ...record };
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    ...$record,
    value: $result
  };
})();
export const nestedMissingValue = (() => {
  const $record = { ...record };
  const $record$1 = { ...record.nested };
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    ...$record,
    nested: {
      ...$record$1,
      value: $result
    }
  };
})();
export const validSibling = (() => {
  const $record = { ...record };
  const $record$1 = { ...record.nested };
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    ...$record,
    nested: {
      ...$record$1,
      value: $result
    },
    value: 3 | 0
  };
})();
