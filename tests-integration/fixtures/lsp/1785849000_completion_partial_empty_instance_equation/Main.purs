module Main where

class Example a where
  member :: a -> a -> a

instance Example Int where
  member first second =
--                     ^
