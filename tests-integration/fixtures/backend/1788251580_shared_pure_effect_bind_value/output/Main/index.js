export function sharedBind($unit) {
  return () => {
    let value;
    value = 42 | 0;
    const $value = {
      left: value,
      right: value
    };
    return $value;
  };
}
