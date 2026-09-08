module Main where

import Data.Ord (class Ord)

data Test = Test Int

derive instance ordTest :: Ord Test
