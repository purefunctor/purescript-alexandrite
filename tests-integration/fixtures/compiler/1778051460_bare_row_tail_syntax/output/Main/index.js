export function bareTail($opts) {
  return 1 | 0;
}
export function plainTail($opts) {
  return 2 | 0;
}
export function recordTail($record) {
  return 3 | 0;
}
export const withBareTail = bareTail({ option: 1 | 0 });
export const withPlainTail = plainTail({ option: 1 | 0 });
export const withRecordTail = recordTail({ option: 1 | 0 });
