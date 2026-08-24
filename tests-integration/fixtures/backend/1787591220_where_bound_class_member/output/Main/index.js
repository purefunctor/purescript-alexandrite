export function use(namedADict) {
  function use$closure(namedADict) {
    return value => {
      return (namedADict => namedADict.name)(namedADict)(value);
    };
  }
  return use$closure(namedADict);
}

export function useLet(namedADict) {
  function useLet$closure(namedADict) {
    return value => {
      return (namedADict => namedADict.name)(namedADict)(value);
    };
  }
  return useLet$closure(namedADict);
}

export const namedString = (() => {
  return { name: value => value };
})();
