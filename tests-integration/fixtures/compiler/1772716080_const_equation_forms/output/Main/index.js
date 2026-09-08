function $const(a) {
  return (b) => {
    return a;
  };
}
export function const2(a) {
  return (b) => a;
}
export function const3(a) {
  return (b) => {
    return a;
  };
}
export { $const as "const" };
