module Main where

data Maybe a = Nothing | Just a

class Convert a where
  convert :: a -> a

instance Convert (Maybe a) where
  convert functionArgument
    | Just instanceGuard <- functionArgument =
-- completion eof
