module Main where

data Maybe a = Nothing | Just a

check input
  | Just firstGuard <- input =
--                            ^
  | Just secondGuard <- input = secondGuard
