export function map(f$1) {
  return (x) => {
    return [];
  };
}
export function apply(f$1) {
  return (x) => {
    return f$1(x);
  };
}
export function f(g) {
  return g({ foo: 1 | 0 });
}
export function test1(section74) {
  return section74.prop;
}
export function test2(section80) {
  return section80.a.b.c;
}
export function test3(section88) {
  return section88.poly;
}
export function test6(r) {
  return (g) => {
    return g((section118) => section118.bar);
  };
}
export function test7(r) {
  return ((section128) => section128.nonexistent)(r);
}
export function test8(section137) {
  return section137((section140) => section140.foo);
}
export function test11(section171) {
  return {
    a: section171,
    b: (section174) => section174.foo
  };
}
export function test12(section180) {
  return [section180, (section182) => section182.bar];
}
export function test13(section189) {
  return (section197) => section197.prop;
}
export function test14(section204) {
  if (section204) {
    return (section207) => section207.a;
  } else {
    return (section211) => section211.b;
  }
}
export const test4 = map((section97) => section97.prop);
export const test5 = f((section106) => section106.foo);
export const test9 = map((section150) => section150((section153) => section153.prop));
export const test10 = apply((section163) => section163.a.b);
