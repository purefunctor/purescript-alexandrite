export function firstClass(value) {
  return value;
}

export const direct = 42 | 0;

export const indirect = firstClass(43 | 0);
