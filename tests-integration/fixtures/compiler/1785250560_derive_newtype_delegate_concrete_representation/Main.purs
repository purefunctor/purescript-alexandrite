module Main where

import Data.Show (class Show)

newtype NonZero = NonZero Int

derive newtype instance Show NonZero
