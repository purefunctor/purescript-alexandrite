export function nestedBind($unit) {
  return () => {
    let value;
    value = 42 | 0;
    return value;
  };
}
