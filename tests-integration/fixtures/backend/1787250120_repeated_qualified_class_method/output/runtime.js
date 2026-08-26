export function binding(name, initialize) {
  let state = 0;
  let value;
  return () => {
    if (state === 2) return value;
    if (state === 1) {
      throw new ReferenceError(`${name} was needed before it finished initializing`);
    }
    state = 1;
    value = initialize();
    state = 2;
    return value;
  };
}
