export const model = { count: 0 | 0, nested: { enabled: true, label: "before" } };

export const updated = (() => {
  const model$1 = model;
  const literal = 1 | 0;
  const literal$1 = false;
  const literal$2 = "after";
  const updated$1 = {
    ...model$1,
    count: literal,
    nested: { ...model$1.nested, enabled: literal$1, label: literal$2 }
  };
  return updated$1;
})();
