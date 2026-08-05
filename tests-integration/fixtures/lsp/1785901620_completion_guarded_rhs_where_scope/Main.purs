module Main where

data Maybe a = Nothing | Just a

check functionBinder
  | Just guardBinder <- functionBinder =
--                                      ^
  where
  whereBinder = guardBinder
