module Main where

import IndexedDo as Indexed

data Quadruple a b c d = Quadruple a b c d

indexed :: Indexed.Render 0 4 (Quadruple Indexed.Unit Indexed.Unit Indexed.Unit Indexed.Unit)
indexed = Indexed.do
  first <- Indexed.action
  second <- Indexed.action
  third <- Indexed.action
  fourth <- Indexed.action
  Indexed.pure (Quadruple first second third fourth)
