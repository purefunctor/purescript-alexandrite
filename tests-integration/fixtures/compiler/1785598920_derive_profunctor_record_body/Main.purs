module Main where

import Data.Profunctor (class Profunctor)

data RecordPro a b = RecordPro { consume :: a -> Int, fixed :: String, produce :: b }
derive instance Profunctor RecordPro
