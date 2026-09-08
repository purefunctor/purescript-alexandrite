module Main where

data Maybe a = Nothing | Just a

nested = \first -> \second -> first

caseBody = \maybe -> case maybe of
  Nothing -> 0
  Just value -> value
