module Main where

data Maybe a = Nothing | Just a

check firstArgument secondArgument
  | Just firstGuard <- firstArgument
  , Just secondGuard <- secondArgument =
-- completion eof
