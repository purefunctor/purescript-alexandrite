module Main where

data Maybe a = Just a | Nothing

foo (Just x) = Just x
--   &
