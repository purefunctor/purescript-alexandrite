module Main where

import Prim.Row as Row

foreign import make :: Row.Lacks "missing" (value :: Int) => Int -> String

test = make 0
