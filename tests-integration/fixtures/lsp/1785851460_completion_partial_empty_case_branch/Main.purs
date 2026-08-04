module Main where

data Maybe a = Just a | Nothing

check input = case input of
  Just value ->
--             ^
