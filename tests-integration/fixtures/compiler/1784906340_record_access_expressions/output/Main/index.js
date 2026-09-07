export function identity(value) {
  return value;
}
export const record = {
  function: identity,
  nested: { value: 42 | 0 }
};
export const direct = record.nested;
export const chained = record.nested.value;
export const functionPosition = record.function(1 | 0);
export const argument = identity(record.nested);
export const applicationBase = identity(record).nested.value;
