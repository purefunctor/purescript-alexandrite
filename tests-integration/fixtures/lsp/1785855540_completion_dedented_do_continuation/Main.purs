module Main where

check outerName = do
  outerDedentedName <- outerName
  do
    innerExcludedName <- outerDedentedName
  
-- completion eof
