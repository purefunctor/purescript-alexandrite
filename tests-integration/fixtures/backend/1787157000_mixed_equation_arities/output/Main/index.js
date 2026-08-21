function choose$closure(value) {
  return value;
}

export function choose(argument0) {
  return argument1 => {
    const matches = argument0 === (0 | 0);
    if (matches) {
      const call = choose$closure(argument1);
      return call;
    } else {
      return argument0;
    }
  };
}
