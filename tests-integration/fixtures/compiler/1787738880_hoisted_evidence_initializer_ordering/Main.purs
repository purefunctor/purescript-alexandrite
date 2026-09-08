module Main where

import Data.Eq as Eq

compareInts :: Int -> Int -> Boolean
compareInts left right =
  if Eq.eq left right then Eq.eq right left else false

initialized :: Boolean
initialized = compareInts 1 1
