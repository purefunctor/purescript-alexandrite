module Main where

unresolvedBind = do
  value <- 1
  value

unresolvedDiscard = do
  1
  2
