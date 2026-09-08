export function test(x) {
  {
    const n = x;
    if (n) {
      return 0 | 0;
    }
    if (true) {
      return n;
    }
  }
  throw new Error("Pattern match failure");
}
