module Main where

class Convert a b | a -> b where
  convert :: a -> b

instance convertSI :: Convert String Int where
  convert _ = 0

instance convertSB :: Convert String Boolean where
  convert _ = true
