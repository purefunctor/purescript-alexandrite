export function choose(first) {
  return second => {
    if (first === true && second === true) {
      return 2 | 0;
    }
    if (first === true && second === false) {
      return 1 | 0;
    }
    if (first === false) {
      return 0 | 0;
    }
    throw new Error("Pattern match failure");
  };
}
