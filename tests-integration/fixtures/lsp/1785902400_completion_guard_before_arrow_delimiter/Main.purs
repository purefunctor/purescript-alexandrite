module Main where

data Maybe a = Nothing | Just a

check input = case input of
  Just caseBinder | Just guardBinder <- input ->
--                                            ^
