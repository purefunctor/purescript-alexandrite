module Main where

import Data.Function.Uncurried (Fn2, mkFn2, runFn2)

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int
foreign import incrementInt :: Int -> Int

uncurriedTail :: Fn2 Int Int Int
uncurriedTail = mkFn2 \value accumulator ->
  if equalInt value 0 then accumulator
  else runFn2 uncurriedTail (decrementInt value) (incrementInt accumulator)
