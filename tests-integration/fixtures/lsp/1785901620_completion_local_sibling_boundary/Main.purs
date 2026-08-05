module Main where

check input = let
  first firstBinder = 0
--                     ^
  second secondBinder = input
  in first
