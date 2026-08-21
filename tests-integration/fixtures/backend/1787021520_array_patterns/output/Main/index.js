export function describe(argument0) {
  if (Array.isArray(argument0) && argument0.length === 0) {
    return 0 | 0;
  } else {
    if (Array.isArray(argument0) && argument0.length === 1) {
      return argument0[0];
    } else {
      if (Array.isArray(argument0) && argument0.length === 2) {
        const first = argument0[0];
        const second = argument0[1];
        return first;
      } else {
        return 3 | 0;
      }
    }
  }
}
