module Main where

check outerName = do
  boundBeforeLetName <- outerName
  let letEquationName = boundBeforeLetName
  let letSignatureName :: Int
      letSignatureName = letEquationName
  outerName
  
-- completion eof
