globalThis.resilientObservations = [];

export const observe = value => {
  globalThis.resilientObservations.push(value);
  return value;
};
