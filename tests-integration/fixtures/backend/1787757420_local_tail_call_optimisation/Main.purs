module Main where

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

localTail :: Int -> Int
localTail value = go value
  where
  go :: Int -> Int
  go current =
    if equalInt current 0 then current
    else go (decrementInt current)
