module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)
import Library (ImportedLifted, ImportedOpaque, ImportedStructural)

derive instance Eq a => Eq (ImportedStructural a)
derive instance Ord a => Ord (ImportedStructural a)

derive instance Eq1 ImportedLifted
derive instance Ord1 ImportedLifted

derive instance Eq1 ImportedOpaque
derive instance Ord1 ImportedOpaque
