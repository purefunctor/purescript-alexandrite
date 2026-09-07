module Main where

import Data.Function as Function
import Lookalike as Lookalike

foreign import observe :: forall value. String -> value -> value
foreign import readTrace :: Boolean -> Array String

directApply :: Int
directApply = Function.apply (\value -> value) 42

directApplyOrder :: Boolean -> Int
directApplyOrder _ =
  Function.apply
    (observe "function" (\value -> value))
    (observe "argument" 42)

flippedApply :: Int
flippedApply = Function.applyFlipped 42 (\value -> value)

flippedApplyOrder :: Boolean -> Int
flippedApplyOrder _ =
  Function.applyFlipped
    (observe "argument" 42)
    (observe "function" (\value -> value))

lookalikeApply :: Int
lookalikeApply = Lookalike.apply (\value -> value) 42
