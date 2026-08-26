module Main where

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int
foreign import incrementInt :: Int -> Int

tailAccumulator :: Int -> Int -> Int
tailAccumulator value accumulator =
  if equalInt value 0 then accumulator
  else tailAccumulator (decrementInt value) (incrementInt accumulator)

rotateArguments :: Int -> Int -> Int -> Int
rotateArguments iterations left right =
  if equalInt iterations 0 then left
  else rotateArguments (decrementInt iterations) right left
