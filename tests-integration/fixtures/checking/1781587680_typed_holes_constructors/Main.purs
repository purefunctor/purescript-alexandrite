module Main where

import Lib

data Option = Yes | No

localOption :: Option
localOption = Yes

option :: Option
option = ?help

imported :: ImportedOption
imported = ?imported

class Select a where
  select :: a -> Option

method :: forall a. Select a => a -> Option
method = ?method
