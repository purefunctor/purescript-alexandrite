export const missingCondition = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  if ($result) {
    return 1 | 0;
  } else {
    return 2 | 0;
  }
})();
export const missingThen = (() => {
  if (true) {
    throw new Error("Generated code reached a source error");
  } else {
    return 2 | 0;
  }
})();
export const missingElse = (() => {
  if (true) {
    return 1 | 0;
  } else {
    throw new Error("Generated code reached a source error");
  }
})();
