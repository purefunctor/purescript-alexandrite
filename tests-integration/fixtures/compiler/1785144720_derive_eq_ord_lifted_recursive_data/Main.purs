module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)

newtype Mu f = In (f (Mu f))

derive instance Eq1 f => Eq (Mu f)
derive instance Ord1 f => Ord (Mu f)
