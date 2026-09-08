export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test1($record) {
  const x = $record.x;
  const y = $record.y;
  return x;
}
export function test2($record) {
  if ($record.x.tag === "Just") {
    const { _1: n } = $record.x;
    return n;
  } else {
    throw new Error("Pattern match failure");
  }
}
export function test3($record) {
  const x = $record.x;
  const y = $record.y;
  return x;
}
export function test4($record) {
  if ($record.a.tag === "Just" && $record.b.tag === "Just") {
    const { _1: n } = $record.a;
    return n;
  } else {
    throw new Error("Pattern match failure");
  }
}
export function test5($record) {
  const x = $record.inner.x;
  return x;
}
