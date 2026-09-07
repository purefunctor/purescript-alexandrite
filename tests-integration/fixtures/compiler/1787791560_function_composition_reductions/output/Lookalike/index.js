export function compose(outer) {
  return (inner) => {
    return (value) => {
      return outer(inner(value));
    };
  };
}
export function composeFlipped(inner) {
  return (outer) => {
    return (value) => {
      return outer(inner(value));
    };
  };
}
