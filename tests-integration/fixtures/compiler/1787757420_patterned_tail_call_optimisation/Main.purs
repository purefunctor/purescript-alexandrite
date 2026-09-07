module Main where

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

patternedTail :: { value :: Int } -> Int
patternedTail { value } =
  if equalInt value 0 then value
  else patternedTail { value: decrementInt value }
