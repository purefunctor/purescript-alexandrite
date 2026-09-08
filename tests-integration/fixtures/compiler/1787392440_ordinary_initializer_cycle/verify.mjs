let failure;
try {
  await import("./output/Main/index.js");
} catch (error) {
  failure = error;
}

if (!(failure instanceof Error) || failure.message !== "Top-level value initializer cycle") {
  throw new Error(`unexpected initializer cycle error: ${failure}`);
}
