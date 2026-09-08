module Main where

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

mutualEven :: Int -> Boolean
mutualEven value =
  if equalInt value 0 then true
  else mutualOdd (decrementInt value)

mutualOdd :: Int -> Boolean
mutualOdd value =
  if equalInt value 0 then false
  else mutualEven (decrementInt value)
