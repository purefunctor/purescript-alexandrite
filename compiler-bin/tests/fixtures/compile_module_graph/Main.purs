module Main where

import Library (answer)

foreign import increment :: Int -> Int

result :: Int
result = increment answer
