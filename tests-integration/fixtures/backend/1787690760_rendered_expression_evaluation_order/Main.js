let trace = [];

export const collect = first => {
  trace.push(`collect:first:${first}`);
  return second => {
    trace.push(`collect:second:${second}`);
    return third => {
      trace.push(`collect:third:${third}`);
      return [first, second, third];
    };
  };
};

export const failAt = label => shouldThrow => value => {
  trace.push(`fail:${label}`);
  if (shouldThrow) throw new Error(label);
  return value;
};

export const observe = label => value => {
  trace.push(`observe:${label}`);
  return value;
};

export const observedRecord = {
  get value() {
    trace.push("read:record");
    return 1;
  },
};

export const readTrace = () => trace.slice();

export const resetTrace = () => {
  trace = [];
};
