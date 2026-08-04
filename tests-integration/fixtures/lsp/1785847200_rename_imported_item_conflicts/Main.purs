module Main where

import Lib (renamed, Renamed)

original = renamed
-- /

data Original = Original Renamed
--   /

localConflict original = renamed + original
--            /
