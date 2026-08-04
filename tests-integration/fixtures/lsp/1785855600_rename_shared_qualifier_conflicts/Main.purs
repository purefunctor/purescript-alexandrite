module Main where

import Lib (original, Original) as Shared
import Other (renamed, Renamed) as Shared

term = Shared.original
--              /

type Example = Shared.Original
--                       /
