export function show(dictionary) {
  return dictionary.show;
}

export function showArray(showADict) {
  return { show: $array => "" };
}

export const showInt = (() => {
  return { show: $int => "" };
})();
