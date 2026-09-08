module Main where

bind x f = f x

test = do
  \ @ -> 1
  3
