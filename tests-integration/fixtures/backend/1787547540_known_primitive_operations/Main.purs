module Main where

import Data.HeytingAlgebra as HeytingAlgebra
import Data.Ring as Ring
import Data.Semiring as Semiring
import Lookalike as Lookalike

foreign import observe :: forall value. String -> value -> value
foreign import readTrace :: Boolean -> Array String

booleanNot :: Boolean -> Boolean
booleanNot value = HeytingAlgebra.not value

integerAdd :: Int -> Int -> Int
integerAdd left right = Semiring.add left right

inlineIntegerAdd :: Int -> Int -> Int
inlineIntegerAdd left right =
  let result = Semiring.add left right
  in result

integerSubtract :: Int -> Int -> Int
integerSubtract left right = Ring.sub left right

integerMultiply :: Int -> Int -> Int
integerMultiply left right = Semiring.mul left right

integerNegate :: Int -> Int
integerNegate value = Ring.negate value

integerNegateLiteral :: Int
integerNegateLiteral = Ring.negate 20

inlineIntegerNegateLiteral :: Int
inlineIntegerNegateLiteral =
  let value = 20
  in Ring.negate value

numberNegate :: Number -> Number
numberNegate value = Ring.negate value

numberNegateLiteral :: Number
numberNegateLiteral = Ring.negate 20.5

integerAddOrder :: Boolean -> Int
integerAddOrder _ =
  Semiring.add
    (observe "left" 20)
    (observe "right" 22)

partiallyAppliedAdd :: Int -> Int
partiallyAppliedAdd = Semiring.add 1

lookalikeAdd :: Int -> Int -> Int
lookalikeAdd left right = Lookalike.add left right
