module Main where

class Before value where
  before :: value -> value -> Boolean

data Marker = Left | Right

instance Before Marker where
  before Left Right = true
  before _ _ = false

runComparison
  :: (forall value. Before value => value -> value -> Boolean)
  -> Boolean
runComparison comparison = comparison Left Right

result :: Boolean
result = runComparison \left right -> before left right
