export function describe(argument0) {
  const matches = Array.isArray(argument0) && argument0.length === 0;
  if (matches) {
    return 0 | 0;
  } else {
    const matches$1 = Array.isArray(argument0) && argument0.length === 1;
    if (matches$1) {
      const value = argument0[0];
      return value;
    } else {
      const matches$2 = Array.isArray(argument0) && argument0.length === 2;
      if (matches$2) {
        const first = argument0[0];
        const second = argument0[1];
        return first;
      } else {
        return 3 | 0;
      }
    }
  }
}
