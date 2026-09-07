module Main where

data Token = FirstToken | SecondToken

class Example a where
  first :: a
  second :: a

instance Example Token where
  second = SecondToken
  first = FirstToken
