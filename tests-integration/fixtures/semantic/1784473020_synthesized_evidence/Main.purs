module Main where

import Data.Symbol (class IsSymbol)

foreign import make :: IsSymbol "hello" => Int -> String

test = make 0
