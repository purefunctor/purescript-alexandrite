export function consume($record) {
  const inner = $record.value;
  return inner;
}
export const value = 42 | 0;
export const empty = {};
export const explicit = {
  z: 1 | 0,
  a: "text"
};
export const punned = { value };
export const nested = {
  array: [1 | 0, 2 | 0],
  record: { boolean: true }
};
export const application = consume({ value: 1 | 0 });
