module Main where

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

singleArgument :: Int -> Int
singleArgument value =
  if equalInt value 0 then value
  else twoArguments (decrementInt value) 0

twoArguments :: Int -> Int -> Int
twoArguments value accumulator =
  if equalInt value 0 then accumulator
  else singleArgument (decrementInt value)
