function $const(a) {
  return ($b) => {
    return a;
  };
}
export function choose($int) {
  return (a) => {
    return ($b) => {
      return a;
    };
  };
}
export const normal = { parenthesised: $const };
export const prenex = {
  variable: choose,
  application: choose(0 | 0)
};
export { $const as "const" };
