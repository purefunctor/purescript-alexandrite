export const model = { count: 0 | 0, nested: { enabled: true, label: "before" } };

export const updated = (() => {
  const updated$1 = {
    ...model,
    count: 1 | 0,
    nested: { ...model.nested, enabled: false, label: "after" }
  };
  return updated$1;
})();
