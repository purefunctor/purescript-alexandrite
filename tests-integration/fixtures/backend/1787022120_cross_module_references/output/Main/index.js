import * as Library from "../Library/index.js";

export function unbox(argument0) {
  const matches = Array.isArray(argument0) && argument0[0] === "Box";
  if (matches) {
    const value = argument0[1];
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
}

export const fromLibrary = unbox(Library.box);

export const directReference = Library.libraryValue;
