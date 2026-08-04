module Main where

import Lib (original, Original)
import Other (renamed, Renamed)

term = original + renamed
--     /

type Example = Original
--             /
