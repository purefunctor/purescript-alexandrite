module Main where

check outerName = do
  outerNestedName <- outerName
  do
    innerNestedName <- outerNestedName
    
-- completion eof
