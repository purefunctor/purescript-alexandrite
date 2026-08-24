export const Unit = ["Unit"];

export function ordinaryUnit(value) {
  return value;
}

export function childArray(parentArrayADict) {
  return { Parent0: () => parentArrayADict, child: value => value };
}

export function useSuperclass(childADict) {
  return value => (childADict.Parent0()).parent(value);
}

export const ordinaryUnitCall = ordinaryUnit(Unit);

export const parentInt = (() => {
  return { parent: value => value };
})();
