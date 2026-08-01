module Main where

import Data.Bifoldable (class Bifoldable)
import Data.Foldable (class Foldable)

data Product a b = Product a b

derive instance Bifoldable Product
derive instance Foldable (Product Int)

data RightNested a = RightNested (Product Int a)

derive instance Foldable RightNested
