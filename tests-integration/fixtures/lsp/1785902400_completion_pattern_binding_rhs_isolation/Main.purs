module Main where

data Maybe a = Nothing | Just a

check outer = earlier
  where
  earlier = outer
  Just patternBinder =
--                    ^
  later laterBinder = outer
