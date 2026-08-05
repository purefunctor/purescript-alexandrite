module Main where

data Maybe a = Nothing | Just a

check input outerArgument = nested
  where
  nested localArgument
    | Just localGuard <- input =
-- completion eof
