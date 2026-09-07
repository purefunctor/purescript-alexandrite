export const Cons = ($value0) => ($value1) => ({
  tag: "Cons",
  _1: $value0,
  _2: $value1
});
export const Nil = "Nil";
export function head($list) {
  if ($list.tag === "Cons") {
    const { _1: x } = $list;
    return x;
  } else {
    throw new Error("Pattern match failure");
  }
}
