module Main where

prefix = "😀"

check outerName = do
  outerBound <- outerName
  do
    innerExcluded <- outerBound
    
--^
