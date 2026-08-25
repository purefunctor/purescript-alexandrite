export const model = { count: 0 | 0, nested: { enabled: true, label: "before" } };

export const updated = (() => {
  const $record = model;
  const $field = 1 | 0;
  const $field$1 = false;
  const $field$2 = "after";
  const $update = {
    ...$record,
    count: $field,
    nested: { ...$record.nested, enabled: $field$1, label: $field$2 }
  };
  return $update;
})();
