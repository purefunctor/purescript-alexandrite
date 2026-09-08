let trace = [];

export const constructEffect = label => value => {
  trace.push(`construct:${label}`);
  return () => {
    trace.push(`run:${label}`);
    return value;
  };
};

export const observe = label => value => {
  trace.push(`observe:${label}`);
  return value;
};

export const observed = {
  get apply() {
    trace.push("read:apply");
    return value => {
      trace.push("apply");
      return value;
    };
  },
  get value() {
    trace.push("read:value");
    return 13;
  },
};

export const readTrace = () => trace.slice();

export const resetTrace = () => {
  trace = [];
};
