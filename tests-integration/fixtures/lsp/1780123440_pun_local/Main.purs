module Main where

tuple { value } = [ value, value ]
--      %           %

record { value } = { value }
--       %           %

letBound = let value = 42 in { value }
--             %               %

binderBound = \value -> { value }
--             %          %
