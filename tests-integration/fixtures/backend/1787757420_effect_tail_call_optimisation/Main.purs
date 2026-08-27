module Main where

import Control.Applicative (pure)
import Control.Bind (discard)
import Data.Unit (Unit)
import Effect (Effect)

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int
foreign import constructTick :: Int -> Effect Unit

effectTail :: Int -> Effect Int
effectTail value =
  if equalInt value 0 then pure value
  else do
    constructTick value
    effectTail (decrementInt value)

effectMutualEven :: Int -> Effect Boolean
effectMutualEven value =
  if equalInt value 0 then pure true
  else do
    constructTick value
    effectMutualOdd (decrementInt value)

effectMutualOdd :: Int -> Effect Boolean
effectMutualOdd value =
  if equalInt value 0 then pure false
  else do
    constructTick value
    effectMutualEven (decrementInt value)

effectMixedShort :: Int -> Effect Int
effectMixedShort value =
  if equalInt value 0 then pure value
  else do
    constructTick value
    effectMixedLong (decrementInt value) 0

effectMixedLong :: Int -> Int -> Effect Int
effectMixedLong value accumulator =
  if equalInt value 0 then pure accumulator
  else do
    constructTick value
    effectMixedShort (decrementInt value)
