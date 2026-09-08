export function add(x) {
  return (y) => {
    return x;
  };
}
export function sub(x) {
  return (y) => {
    return x;
  };
}
export function mul(x) {
  return (y) => {
    return x;
  };
}
function $const(x) {
  return (y) => {
    return x;
  };
}
export function apply(f) {
  return (x) => {
    return f(x);
  };
}
export function chain(g) {
  return (f) => {
    return (x) => {
      return g(f(x));
    };
  };
}
export function curry(f) {
  return (x) => {
    return (y) => {
      return f(x)(y);
    };
  };
}
export const test1 = add(1 | 0)(2 | 0);
const test1_ = add(1 | 0)(2 | 0);
export const test2 = sub(add(1 | 0)(2 | 0))(3 | 0);
const test2_ = sub(add(1 | 0)(2 | 0))(3 | 0);
export const test3 = mul(sub(add(1 | 0)(2 | 0))(3 | 0))(4 | 0);
const test3_ = mul(sub(add(1 | 0)(2 | 0))(3 | 0))(4 | 0);
export const test4 = $const(1 | 0)("hello");
const test4_ = $const(1 | 0)("hello");
export const test5 = add($const(1 | 0)("hello"))(2 | 0);
const test5_ = add($const(1 | 0)("hello"))(2 | 0);
export const test6 = apply(chain((x) => x)((x$1) => x$1))(1 | 0);
const test6_ = apply(chain((x) => x)((x$1) => x$1))(1 | 0);
export const test7 = curry(add)(1 | 0);
const test7_ = curry(add)(1 | 0);
export { test1_ as "test1'" };
export { test2_ as "test2'" };
export { test3_ as "test3'" };
export { $const as "const" };
export { test4_ as "test4'" };
export { test5_ as "test5'" };
export { test6_ as "test6'" };
export { test7_ as "test7'" };
