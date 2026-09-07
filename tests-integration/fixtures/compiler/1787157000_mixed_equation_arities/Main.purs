module Main where

choose :: Int -> Int -> Int
choose 0 = \value -> value
choose left _ = left
