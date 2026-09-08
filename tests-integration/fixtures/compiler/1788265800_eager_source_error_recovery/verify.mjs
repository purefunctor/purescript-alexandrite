let failure;
try {
  await import("./output/Main/index.js");
} catch (error) {
  failure = error;
}

if (!(failure instanceof Error) || failure.message !== "Generated code reached a source error") {
  throw new Error(`unexpected eager source error: ${failure}`);
}

if (JSON.stringify(globalThis.resilientObservations) !== "[1]") {
  throw new Error(`unexpected evaluation order: ${globalThis.resilientObservations}`);
}
