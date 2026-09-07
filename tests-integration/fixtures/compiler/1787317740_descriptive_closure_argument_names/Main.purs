module Main where

data Choice = None | Some Int

data Box = Box Int

multiEquation :: Choice -> Int
multiEquation None = 0
multiEquation (Some value) = value

mixedArity :: Boolean -> Int -> Int
mixedArity true = \value -> value
mixedArity false value = value

singleConstructor :: Box -> Int
singleConstructor (Box value) = value

singleWildcards :: Int -> Int -> Boolean
singleWildcards _ _ = true

functionWildcard :: (Int -> Int) -> Int
functionWildcard _ = 0

rigidWildcard :: forall value. value -> Boolean
rigidWildcard _ = true

namedPattern :: { value :: Int } -> { value :: Int }
namedPattern record@{ value: _ } = record

capture :: Int -> Boolean -> Int
capture captured _ = captured
