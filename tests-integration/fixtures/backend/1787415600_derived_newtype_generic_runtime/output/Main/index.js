import * as Data_Generic_Rep from "../Data.Generic.Rep/index.js";
import * as Data_Newtype from "../Data.Newtype/index.js";
import * as $runtime from "../runtime.js";

export const Empty = ["Empty"];
export const Single = $value0 => ["Single", $value0];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];

export function roundTrip(value) {
  return genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt.to(
    genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt.from(
      value
    )
  );
}

const $lazy_genericVoidNoConstructors = $runtime.binding("genericVoidNoConstructors", () => {
  function genericVoidNoConstructors$initialize$closure(value) {
    return ($lazy_genericVoidNoConstructors()).to(value);
  }
  function genericVoidNoConstructors$initialize$closure$1(value) {
    return ($lazy_genericVoidNoConstructors()).from(value);
  }
  return {
    to: genericVoidNoConstructors$initialize$closure,
    from: genericVoidNoConstructors$initialize$closure$1
  };
});

export const newtypeTypeIdentifierInt = (() => {
  function newtypeTypeIdentifierInt$initialize$closure(unit) {
    return {};
  }
  return { Coercible0: newtypeTypeIdentifierInt$initialize$closure };
})();

export const wrapped = 42 | 0;

export const unwrapped = Data_Newtype.unwrap(newtypeTypeIdentifierInt)(wrapped);

export const genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt = (() => {
  function genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt$initialize$closure(representation) {
    function case$1(representation) {
      if (Array.isArray(representation) && representation[0] === "Inr") {
        const argument$2 = representation[1];
        if (Array.isArray(argument$2) && argument$2[0] === "Inl") {
          const argument$3 = argument$2[1];
          if (Array.isArray(argument$3) && argument$3[0] === "Constructor") {
            return Single(argument$3[1]);
          } else {
            return case$2(representation);
          }
        } else {
          return case$2(representation);
        }
      } else {
        return case$2(representation);
      }
    }

    function case$2(representation) {
      if (Array.isArray(representation) && representation[0] === "Inr") {
        const argument$4 = representation[1];
        if (Array.isArray(argument$4) && argument$4[0] === "Inr") {
          const argument$5 = argument$4[1];
          if (Array.isArray(argument$5) && argument$5[0] === "Constructor") {
            const argument$6 = argument$5[1];
            if (Array.isArray(argument$6) && argument$6[0] === "Product") {
              const field0$1 = argument$6[1];
              const field1 = argument$6[2];
              return Pair(field0$1)(field1);
            } else {
              throw new Error("Pattern match failure");
            }
          } else {
            throw new Error("Pattern match failure");
          }
        } else {
          throw new Error("Pattern match failure");
        }
      } else {
        throw new Error("Pattern match failure");
      }
    }

    if (Array.isArray(representation) && representation[0] === "Inl") {
      const argument = representation[1];
      if (Array.isArray(argument) && argument[0] === "Constructor") {
        const argument$1 = argument[1];
        if (Array.isArray(argument$1) && argument$1[0] === "NoArguments") {
          return Empty;
        } else {
          return case$1(representation);
        }
      } else {
        return case$1(representation);
      }
    } else {
      return case$1(representation);
    }
  }
  function genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt$initialize$closure$1(value) {
    if (Array.isArray(value) && value[0] === "Empty") {
      return Data_Generic_Rep.Inl(Data_Generic_Rep.Constructor(Data_Generic_Rep.NoArguments));
    } else {
      if (Array.isArray(value) && value[0] === "Single") {
        return Data_Generic_Rep.Inr(Data_Generic_Rep.Inl(Data_Generic_Rep.Constructor(value[1])));
      } else {
        if (Array.isArray(value) && value[0] === "Pair") {
          const field0$1 = value[1];
          const field1 = value[2];
          return Data_Generic_Rep.Inr(
            Data_Generic_Rep.Inr(
              Data_Generic_Rep.Constructor(Data_Generic_Rep.Product(field0$1)(field1))
            )
          );
        } else {
          throw new Error("Pattern match failure");
        }
      }
    }
  }
  return {
    to: genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt$initialize$closure,
    from: genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt$initialize$closure$1
  };
})();

export const emptyRoundTrip = roundTrip(Empty);

export const singleRoundTrip = roundTrip(Single(6 | 0));

export const pairRoundTrip = roundTrip(Pair(7 | 0)(8 | 0));

export const genericVoidNoConstructors = $lazy_genericVoidNoConstructors();
