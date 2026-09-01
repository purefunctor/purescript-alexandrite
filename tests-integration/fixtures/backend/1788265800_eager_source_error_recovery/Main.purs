module Main where

foreign import observe :: Int -> Int

broken :: Array Int
broken = [observe 1, missing]
