module Main where

check outerName = do
  outerBound <- outerName
  do
    innerBound <- outerBound
      
--  ^
