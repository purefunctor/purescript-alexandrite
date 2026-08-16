module Main where

class Convert :: Type -> Constraint
class Convert value where
  convert :: value -> Int

convertInt :: Int
convertInt = 0

instance Convert Int where
  convert value = value

test :: Int
test = convert 42
