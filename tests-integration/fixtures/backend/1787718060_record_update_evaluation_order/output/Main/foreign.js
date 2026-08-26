let trace = [];

export const symbolKey = Symbol("marker");

const tracedRecord = (label, fields) =>
  new Proxy(fields, {
    ownKeys(target) {
      trace.push(`${label}:keys`);
      return Reflect.ownKeys(target);
    },
    getOwnPropertyDescriptor(target, property) {
      trace.push(`${label}:descriptor:${String(property)}`);
      return Reflect.getOwnPropertyDescriptor(target, property);
    },
    get(target, property, receiver) {
      trace.push(`${label}:get:${String(property)}`);
      return Reflect.get(target, property, receiver);
    },
  });

export const makeModel = label => {
  trace.push(`make:${label}`);
  const nested = tracedRecord(`${label}.nested`, {
    value: 2,
    untouched: 5,
  });
  return tracedRecord(label, {
    first: 1,
    nested,
    last: 3,
    untouched: 4,
    [symbolKey]: "symbol-value",
  });
};

export const observe = label => value => {
  trace.push(`observe:${label}`);
  return value;
};

export const failAt = label => shouldThrow => value => {
  trace.push(`fail:${label}`);
  if (shouldThrow) throw new Error(label);
  return value;
};

export const readTrace = () => trace.slice();

export const resetTrace = () => {
  trace = [];
};
