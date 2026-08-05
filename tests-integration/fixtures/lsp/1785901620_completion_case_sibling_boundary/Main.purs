module Main where

data Maybe a = Nothing | Just a

check input = case input of
  Just firstCase -> 0
--                   ^
  Just secondCase -> 1
