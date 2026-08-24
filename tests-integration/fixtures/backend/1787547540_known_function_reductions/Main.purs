module Main where

import Control.Category as Category
import Data.Function as Function
import Lookalike as Lookalike
import Unsafe.Coerce as UnsafeCoerce

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

functionIdentity :: Int
functionIdentity = Category.identity 42

coerced :: Int
coerced = UnsafeCoerce.unsafeCoerce 42

lookalikeApply :: Int
lookalikeApply = Lookalike.apply (\value -> value) 42

lookalikeIdentity :: Int
lookalikeIdentity = Lookalike.identity 42

lookalikeCoerce :: Int
lookalikeCoerce = Lookalike.unsafeCoerce 42
