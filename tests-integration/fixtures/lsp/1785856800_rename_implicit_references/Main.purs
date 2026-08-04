module Main where

map function value = function value
--/

topLevelMap = ado
  value <- [1]
  in value

localMap map = ado
--       /
  value <- [1]
  in value

localBind bind = do
--        /
  value <- [1]
  value

localNegate negate value = -value
--          /
