module Main where

import Data.Show (class Show)

newtype NonEmpty a = NonEmpty (Array a)

derive newtype instance Show a => Show (NonEmpty a)
