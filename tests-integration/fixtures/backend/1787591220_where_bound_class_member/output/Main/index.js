export function use(namedADict) {
  return value => (namedADict => namedADict.name)(namedADict)(value);
}

export function useLet(namedADict) {
  return value => (namedADict => namedADict.name)(namedADict)(value);
}

export const namedString = (() => {
  return { name: value => value };
})();
